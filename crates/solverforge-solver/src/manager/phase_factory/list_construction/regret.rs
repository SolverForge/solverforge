use std::cmp::Reverse;
use std::collections::HashMap;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::state::ScoredConstructionState;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepControlPolicy, StepScope};

const OWNER_RESTRICTED_REGRET_TRIAL_BUDGET: usize = 16_384;
const OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET: usize = 8_000_000;

/// List construction phase using regret-based insertion.
///
/// Extends cheapest insertion by selecting the element with the **highest regret**
/// at each step. Regret is defined as the score difference between the best and
/// second-best insertion positions for an element.
///
/// Inserting high-regret elements first prevents "greedy theft" where easy elements
/// consume the best slots before harder-to-place elements are considered.
///
/// # Algorithm
///
/// ```text
/// while there are unassigned elements:
///     for each unassigned element e:
///         find best insertion (score_1, position_1)
///         find second-best insertion (score_2, position_2)
///         regret(e) = score_1 - score_2   (higher = more urgent)
///     select element e* with maximum regret
///     permanently insert e* at position_1(e*)
/// ```
///
/// Complexity: O(E² × N × M) — quadratic in elements because we re-evaluate
/// all remaining elements each step. This is more expensive than cheapest
/// insertion but produces better solutions.
///
/// # Example
///
/// ```
/// use solverforge_solver::ListRegretInsertionPhase;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan { vehicles: Vec<Vehicle>, n_visits: usize, score: Option<SoftScore> }
///
/// impl PlanningSolution for Plan {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Plan, e: usize) -> usize {
///     s.vehicles.get(e).map_or(0, |v| v.visits.len())
/// }
/// fn list_insert(s: &mut Plan, e: usize, pos: usize, val: usize) {
///     if let Some(v) = s.vehicles.get_mut(e) { v.visits.insert(pos, val); }
/// }
/// fn list_remove(s: &mut Plan, e: usize, pos: usize) -> usize {
///     s.vehicles.get_mut(e).map(|v| v.visits.remove(pos)).unwrap_or(0)
/// }
///
/// let phase = ListRegretInsertionPhase::<Plan, usize>::new(
///     |p| p.n_visits,
///     |p| p.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect(),
///     |p| p.vehicles.len(),
///     list_len,
///     list_insert,
///     list_remove,
///     |_plan, idx| idx,
///     0,
/// );
/// ```
pub struct ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    state: ScoredConstructionState<S, E>,
    _marker: PhantomData<fn() -> (S, E)>,
}

#[derive(Debug, PartialEq, Eq)]
enum RegretValue<Sc> {
    Finite(Sc),
    Forced,
}

impl<Sc: Copy> Copy for RegretValue<Sc> {}

impl<Sc: Copy> Clone for RegretValue<Sc> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Sc: Ord> PartialOrd for RegretValue<Sc> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Sc: Ord> Ord for RegretValue<Sc> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (RegretValue::Forced, RegretValue::Forced) => std::cmp::Ordering::Equal,
            (RegretValue::Forced, RegretValue::Finite(_)) => std::cmp::Ordering::Greater,
            (RegretValue::Finite(_), RegretValue::Forced) => std::cmp::Ordering::Less,
            (RegretValue::Finite(left), RegretValue::Finite(right)) => left.cmp(right),
        }
    }
}

fn regret_choice_is_better<Sc: Copy + Ord>(
    regret: RegretValue<Sc>,
    score: Sc,
    best_regret: RegretValue<Sc>,
    best_score: Sc,
) -> bool {
    regret > best_regret || (regret == best_regret && score > best_score)
}

fn regret_choice_is_better_with_downstream<Sc: Copy + Ord>(
    regret: RegretValue<Sc>,
    score: Sc,
    downstream: usize,
    best_regret: RegretValue<Sc>,
    best_score: Sc,
    best_downstream: usize,
) -> bool {
    regret_choice_is_better(regret, score, best_regret, best_score)
        || (regret == best_regret && score == best_score && downstream > best_downstream)
}

fn precedence_frontier_choice_is_better<Sc: Copy + Ord>(
    downstream: usize,
    regret: RegretValue<Sc>,
    score: Sc,
    best_downstream: usize,
    best_regret: RegretValue<Sc>,
    best_score: Sc,
) -> bool {
    downstream > best_downstream
        || (downstream == best_downstream
            && regret_choice_is_better(regret, score, best_regret, best_score))
}

fn owner_restricted_regret_trial_count(bucket_sizes: &[usize]) -> usize {
    bucket_sizes.iter().fold(0usize, |total, &len| {
        let owner_trials = len
            .saturating_mul(len.saturating_add(1))
            .saturating_mul(len.saturating_add(2))
            / 6;
        total.saturating_add(owner_trials)
    })
}

fn owner_restricted_best_insertion_trial_count(bucket_sizes: &[usize]) -> usize {
    bucket_sizes.iter().fold(0usize, |total, &len| {
        total.saturating_add(len.saturating_mul(len.saturating_add(1)) / 2)
    })
}

fn topological_order(
    successors: &[Vec<usize>],
    predecessor_counts: &[usize],
) -> Option<Vec<usize>> {
    let mut predecessor_counts = predecessor_counts.to_vec();
    let mut ready = predecessor_counts
        .iter()
        .enumerate()
        .filter_map(|(idx, &count)| (count == 0).then_some(idx))
        .collect::<Vec<_>>();
    let mut order = Vec::with_capacity(successors.len());

    while let Some(idx) = ready.pop() {
        order.push(idx);
        for &successor in &successors[idx] {
            predecessor_counts[successor] -= 1;
            if predecessor_counts[successor] == 0 {
                ready.push(successor);
            }
        }
    }

    (order.len() == successors.len()).then_some(order)
}

fn precedence_frontier_regret_trial_count(
    successors: &[Vec<usize>],
    predecessor_counts: &[usize],
    owners: &[usize],
    owner_count: usize,
) -> Option<usize> {
    if successors.len() != predecessor_counts.len() || successors.len() != owners.len() {
        return None;
    }
    let mut predecessor_counts = predecessor_counts.to_vec();
    let mut ready = predecessor_counts
        .iter()
        .enumerate()
        .filter_map(|(idx, &count)| (count == 0).then_some(idx))
        .collect::<Vec<_>>();
    let mut owner_lengths = vec![0usize; owner_count];
    let mut processed = 0usize;
    let mut trials = 0usize;

    while let Some(idx) = ready.pop() {
        for &ready_idx in &ready {
            let owner_idx = *owners.get(ready_idx)?;
            if owner_idx >= owner_count {
                return None;
            }
            trials = trials.saturating_add(owner_lengths[owner_idx].saturating_add(1));
        }
        let owner_idx = *owners.get(idx)?;
        if owner_idx >= owner_count {
            return None;
        }
        trials = trials.saturating_add(owner_lengths[owner_idx].saturating_add(1));
        owner_lengths[owner_idx] = owner_lengths[owner_idx].saturating_add(1);
        processed += 1;
        for &successor in &successors[idx] {
            predecessor_counts[successor] -= 1;
            if predecessor_counts[successor] == 0 {
                ready.push(successor);
            }
        }
    }

    (processed == successors.len()).then_some(trials)
}

fn downstream_durations(
    successors: &[Vec<usize>],
    durations: &[usize],
    topological_order: &[usize],
) -> Vec<usize> {
    let mut downstream = durations.to_vec();
    for &idx in topological_order.iter().rev() {
        let successor_tail = successors[idx]
            .iter()
            .map(|&successor| downstream[successor])
            .max()
            .unwrap_or(0);
        downstream[idx] = durations[idx].saturating_add(successor_tail);
    }
    downstream
}

impl<S, E> std::fmt::Debug for ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListRegretInsertionPhase").finish()
    }
}

impl<S, E> ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    /* Creates a new regret insertion phase.

    # Arguments
    * `element_count` - Total number of elements to assign
    * `get_assigned` - Returns currently assigned elements
    * `entity_count` - Total number of entities (routes/vehicles)
    * `list_len` - Length of entity's list
    * `list_insert` - Insert element at position (shifts right)
    * `list_remove` - Remove element at position (used for undo), returns removed element
    * `index_to_element` - Converts element index to element value
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
        list_remove: fn(&mut S, usize, usize) -> E,
        index_to_element: fn(&S, usize) -> E,
        descriptor_index: usize,
    ) -> Self {
        Self {
            state: ScoredConstructionState {
                element_count,
                get_assigned,
                entity_count,
                list_len,
                list_insert,
                list_remove,
                index_to_element,
                element_owner_fn: None,
                element_order_key: None,
                precedence_duration_fn: None,
                precedence_successors_fn: None,
                descriptor_index,
            },
            _marker: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    ) -> Self {
        self.state.element_owner_fn = element_owner_fn;
        self
    }

    pub fn with_element_order_key(mut self, element_order_key: Option<fn(&S, E) -> i64>) -> Self {
        self.state.element_order_key = element_order_key;
        self
    }

    pub(crate) fn with_precedence_hooks(
        mut self,
        duration_fn: Option<fn(&S, E) -> usize>,
        successors_fn: Option<fn(&S, E, &mut Vec<E>)>,
    ) -> Self {
        self.state.precedence_duration_fn = duration_fn;
        self.state.precedence_successors_fn = successors_fn;
        self
    }

    /* Evaluate best and second-best insertions for `element`. */
    fn evaluate_regret<D: Director<S>>(
        &self,
        element: E,
        n_entities: usize,
        score_director: &mut D,
    ) -> Option<(RegretValue<S::Score>, usize, usize, S::Score)> {
        let list_len = self.state.list_len;
        let mut best: Option<(usize, usize, S::Score)> = None;
        let mut second_best: Option<S::Score> = None;

        let solution = score_director.working_solution();
        let candidates = crate::list_placement::candidate_entity_indices(
            self.state.element_owner_fn,
            solution,
            n_entities,
            &element,
        );
        for entity_idx in candidates {
            let len = list_len(score_director.working_solution(), entity_idx);
            for pos in 0..=len {
                if let Some(score) =
                    self.state
                        .eval_insertion(element, entity_idx, pos, score_director)
                {
                    match best {
                        None => best = Some((entity_idx, pos, score)),
                        Some((_, _, best_score)) if score > best_score => {
                            second_best = Some(best_score);
                            best = Some((entity_idx, pos, score));
                        }
                        Some(_) => match second_best {
                            None => second_best = Some(score),
                            Some(existing_second) if score > existing_second => {
                                second_best = Some(score);
                            }
                            Some(_) => {}
                        },
                    }
                }
            }
        }

        let (best_entity, best_pos, best_score) = best?;

        let regret = match second_best {
            Some(second_score) => RegretValue::Finite(best_score - second_score),
            None => RegretValue::Forced,
        };

        Some((regret, best_entity, best_pos, best_score))
    }

    /* Evaluate best and second-best insertions for `element` into its fixed owner. */
    fn evaluate_owner_regret<D: Director<S>>(
        &self,
        element: E,
        owner_idx: usize,
        score_director: &mut D,
    ) -> Option<(RegretValue<S::Score>, usize, S::Score)> {
        let len = (self.state.list_len)(score_director.working_solution(), owner_idx);
        let mut best: Option<(usize, S::Score)> = None;
        let mut second_best: Option<S::Score> = None;

        for pos in 0..=len {
            if let Some(score) = self
                .state
                .eval_insertion(element, owner_idx, pos, score_director)
            {
                match best {
                    None => best = Some((pos, score)),
                    Some((_, best_score)) if score > best_score => {
                        second_best = Some(best_score);
                        best = Some((pos, score));
                    }
                    Some(_) => match second_best {
                        None => second_best = Some(score),
                        Some(existing_second) if score > existing_second => {
                            second_best = Some(score);
                        }
                        Some(_) => {}
                    },
                }
            }
        }

        let (best_pos, best_score) = best?;
        let regret = match second_best {
            Some(second_score) => RegretValue::Finite(best_score - second_score),
            None => RegretValue::Forced,
        };

        Some((regret, best_pos, best_score))
    }

    fn owner_restricted_bucket_sizes(
        &self,
        solution: &S,
        unassigned: &[E],
        n_entities: usize,
    ) -> Option<Vec<usize>> {
        let owner_fn = self.state.element_owner_fn?;
        let mut bucket_sizes = vec![0; n_entities];
        for element in unassigned.iter().copied() {
            let owner_idx = owner_fn(solution, &element)?;
            if owner_idx >= n_entities {
                return None;
            }
            bucket_sizes[owner_idx] += 1;
        }
        Some(bucket_sizes)
    }

    fn precedence_downstream_by_element(
        &self,
        solution: &S,
        unassigned: &[E],
    ) -> Option<HashMap<E, usize>> {
        let duration_fn = self.state.precedence_duration_fn?;
        let successors_fn = self.state.precedence_successors_fn?;
        if unassigned.is_empty() {
            return Some(HashMap::new());
        }

        let mut index_by_element = HashMap::with_capacity(unassigned.len());
        for (idx, &element) in unassigned.iter().enumerate() {
            if index_by_element.insert(element, idx).is_some() {
                return None;
            }
        }

        let durations = unassigned
            .iter()
            .map(|&element| duration_fn(solution, element))
            .collect::<Vec<_>>();
        let mut successors = vec![Vec::new(); unassigned.len()];
        let mut predecessor_counts = vec![0usize; unassigned.len()];
        let mut scratch = Vec::new();
        for (from_idx, &element) in unassigned.iter().enumerate() {
            scratch.clear();
            successors_fn(solution, element, &mut scratch);
            for successor in &scratch {
                if let Some(&to_idx) = index_by_element.get(successor) {
                    successors[from_idx].push(to_idx);
                    predecessor_counts[to_idx] += 1;
                }
            }
        }

        let topo = topological_order(&successors, &predecessor_counts)?;
        let downstream = downstream_durations(&successors, &durations, &topo);
        Some(
            unassigned
                .iter()
                .copied()
                .zip(downstream)
                .collect::<HashMap<_, _>>(),
        )
    }

    fn solve_oversized_owner_restricted<'t, 'a, D, BestCb>(
        &self,
        phase_scope: &mut PhaseScope<'t, 'a, S, D, BestCb>,
        unassigned: &[E],
        n_entities: usize,
    ) -> bool
    where
        D: Director<S>,
        BestCb: crate::scope::ProgressCallback<S>,
    {
        let Some(bucket_sizes) = self.owner_restricted_bucket_sizes(
            phase_scope.score_director().working_solution(),
            unassigned,
            n_entities,
        ) else {
            return false;
        };

        if owner_restricted_regret_trial_count(&bucket_sizes) > OWNER_RESTRICTED_REGRET_TRIAL_BUDGET
        {
            if self.solve_precedence_frontier_regret(phase_scope, unassigned, n_entities) {
                return true;
            }
            if let Some(dispatch_order) = self.precedence_dispatch_order(
                phase_scope.score_director().working_solution(),
                unassigned,
                n_entities,
            ) {
                self.apply_owner_ordered_append(phase_scope, &dispatch_order);
                return true;
            }
            let estimated_work = owner_restricted_best_insertion_trial_count(&bucket_sizes)
                .saturating_mul(unassigned.len());
            if estimated_work <= OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET {
                self.solve_owner_ordered_best_insertion(phase_scope, unassigned, n_entities);
            } else {
                self.solve_owner_ordered_append(phase_scope, unassigned, n_entities);
            }
            return true;
        }
        false
    }

    fn solve_precedence_frontier_regret<'t, 'a, D, BestCb>(
        &self,
        phase_scope: &mut PhaseScope<'t, 'a, S, D, BestCb>,
        unassigned: &[E],
        n_entities: usize,
    ) -> bool
    where
        D: Director<S>,
        BestCb: crate::scope::ProgressCallback<S>,
    {
        let Some(owner_fn) = self.state.element_owner_fn else {
            return false;
        };
        let Some(duration_fn) = self.state.precedence_duration_fn else {
            return false;
        };
        let Some(successors_fn) = self.state.precedence_successors_fn else {
            return false;
        };
        if unassigned.is_empty() {
            return true;
        }

        let solution = phase_scope.score_director().working_solution();
        let mut index_by_element = HashMap::with_capacity(unassigned.len());
        for (idx, &element) in unassigned.iter().enumerate() {
            if index_by_element.insert(element, idx).is_some() {
                return false;
            }
        }

        let mut owners = Vec::with_capacity(unassigned.len());
        let mut durations = Vec::with_capacity(unassigned.len());
        for &element in unassigned {
            let Some(owner_idx) = owner_fn(solution, &element).filter(|owner| *owner < n_entities)
            else {
                return false;
            };
            owners.push(owner_idx);
            durations.push(duration_fn(solution, element));
        }

        let mut successors = vec![Vec::new(); unassigned.len()];
        let mut predecessor_counts = vec![0usize; unassigned.len()];
        let mut scratch = Vec::new();
        for (from_idx, &element) in unassigned.iter().enumerate() {
            scratch.clear();
            successors_fn(solution, element, &mut scratch);
            for successor in &scratch {
                if let Some(&to_idx) = index_by_element.get(successor) {
                    successors[from_idx].push(to_idx);
                    predecessor_counts[to_idx] += 1;
                }
            }
        }

        let Some(frontier_trials) = precedence_frontier_regret_trial_count(
            &successors,
            &predecessor_counts,
            &owners,
            n_entities,
        ) else {
            return false;
        };
        if frontier_trials.saturating_mul(unassigned.len())
            > OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET
        {
            return false;
        }
        let Some(topo) = topological_order(&successors, &predecessor_counts) else {
            return false;
        };
        let downstream = downstream_durations(&successors, &durations, &topo);
        let mut remaining_predecessors = predecessor_counts;
        let mut ready = topo
            .iter()
            .copied()
            .filter(|&idx| remaining_predecessors[idx] == 0)
            .collect::<Vec<_>>();
        let mut inserted = 0usize;

        while !ready.is_empty() {
            if phase_scope
                .solver_scope_mut()
                .should_interrupt_mandatory_construction()
            {
                break;
            }

            let mut best_choice: Option<(
                RegretValue<S::Score>,
                S::Score,
                usize,
                usize,
                usize,
                usize,
            )> = None;
            let mut interrupted = false;

            for (ready_pos, &idx) in ready.iter().enumerate() {
                if phase_scope
                    .solver_scope_mut()
                    .should_interrupt_mandatory_construction()
                {
                    interrupted = true;
                    break;
                }
                let element = unassigned[idx];
                let owner_idx = owners[idx];
                let Some((regret, pos, score)) = self.evaluate_owner_regret(
                    element,
                    owner_idx,
                    phase_scope.score_director_mut(),
                ) else {
                    continue;
                };
                let is_better = match best_choice {
                    None => true,
                    Some((best_regret, best_score, best_downstream, _, _, _)) => {
                        precedence_frontier_choice_is_better(
                            downstream[idx],
                            regret,
                            score,
                            best_downstream,
                            best_regret,
                            best_score,
                        )
                    }
                };
                if is_better {
                    best_choice = Some((regret, score, downstream[idx], ready_pos, idx, pos));
                }
            }

            if interrupted {
                break;
            }

            let Some((_, score, _, ready_pos, idx, pos)) = best_choice else {
                return false;
            };
            ready.remove(ready_pos);
            let element = unassigned[idx];
            let owner_idx = owners[idx];

            let mut step_scope = StepScope::new_with_control_policy(
                phase_scope,
                StepControlPolicy::CompleteMandatoryConstruction,
            );

            step_scope.apply_committed_change(|sd| {
                self.state.apply_insertion(element, owner_idx, pos, sd);
            });

            step_scope.set_step_score(score);
            step_scope.complete();
            inserted += 1;

            for &successor in &successors[idx] {
                remaining_predecessors[successor] -= 1;
                if remaining_predecessors[successor] == 0 {
                    ready.push(successor);
                }
            }
        }

        inserted == unassigned.len()
    }

    fn solve_owner_ordered_append<'t, 'a, D, BestCb>(
        &self,
        phase_scope: &mut PhaseScope<'t, 'a, S, D, BestCb>,
        unassigned: &[E],
        n_entities: usize,
    ) where
        D: Director<S>,
        BestCb: crate::scope::ProgressCallback<S>,
    {
        let Some(owner_fn) = self.state.element_owner_fn else {
            return;
        };

        if let Some(dispatch_order) = self.precedence_dispatch_order(
            phase_scope.score_director().working_solution(),
            unassigned,
            n_entities,
        ) {
            self.apply_owner_ordered_append(phase_scope, &dispatch_order);
            return;
        }

        let mut owned_order = Vec::with_capacity(unassigned.len());
        for &element in unassigned {
            let solution = phase_scope.score_director().working_solution();
            let Some(owner_idx) = owner_fn(solution, &element).filter(|owner| *owner < n_entities)
            else {
                tracing::warn!("No valid owner found for owner-restricted regret fallback element");
                continue;
            };
            owned_order.push((element, owner_idx));
        }
        self.apply_owner_ordered_append(phase_scope, &owned_order);
    }

    fn apply_owner_ordered_append<'t, 'a, D, BestCb>(
        &self,
        phase_scope: &mut PhaseScope<'t, 'a, S, D, BestCb>,
        owned_order: &[(E, usize)],
    ) where
        D: Director<S>,
        BestCb: crate::scope::ProgressCallback<S>,
    {
        for &(element, owner_idx) in owned_order {
            if phase_scope
                .solver_scope_mut()
                .should_interrupt_mandatory_construction()
            {
                break;
            }

            let solution = phase_scope.score_director().working_solution();
            let pos = (self.state.list_len)(solution, owner_idx);

            let mut step_scope = StepScope::new_with_control_policy(
                phase_scope,
                StepControlPolicy::CompleteMandatoryConstruction,
            );

            step_scope.apply_committed_change(|sd| {
                self.state.apply_insertion(element, owner_idx, pos, sd);
            });

            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();
        }
    }

    fn precedence_dispatch_order(
        &self,
        solution: &S,
        unassigned: &[E],
        n_entities: usize,
    ) -> Option<Vec<(E, usize)>> {
        let owner_fn = self.state.element_owner_fn?;
        let duration_fn = self.state.precedence_duration_fn?;
        let successors_fn = self.state.precedence_successors_fn?;
        if unassigned.is_empty() {
            return Some(Vec::new());
        }

        let mut index_by_element = HashMap::with_capacity(unassigned.len());
        for (idx, &element) in unassigned.iter().enumerate() {
            if index_by_element.insert(element, idx).is_some() {
                return None;
            }
        }

        let mut owners = Vec::with_capacity(unassigned.len());
        let mut durations = Vec::with_capacity(unassigned.len());
        for &element in unassigned {
            let owner_idx = owner_fn(solution, &element)?;
            if owner_idx >= n_entities {
                return None;
            }
            owners.push(owner_idx);
            durations.push(duration_fn(solution, element));
        }

        let mut successors = vec![Vec::new(); unassigned.len()];
        let mut predecessor_counts = vec![0usize; unassigned.len()];
        let mut scratch = Vec::new();
        for (from_idx, &element) in unassigned.iter().enumerate() {
            scratch.clear();
            successors_fn(solution, element, &mut scratch);
            for successor in &scratch {
                if let Some(&to_idx) = index_by_element.get(successor) {
                    successors[from_idx].push(to_idx);
                    predecessor_counts[to_idx] += 1;
                }
            }
        }

        let topo = topological_order(&successors, &predecessor_counts)?;
        let downstream = downstream_durations(&successors, &durations, &topo);
        let order_key_fn = self.state.element_order_key;
        let mut remaining_predecessors = predecessor_counts;
        let mut predecessor_ready = vec![0usize; unassigned.len()];
        let mut owner_ready = vec![0usize; n_entities];
        let mut ready = topo
            .iter()
            .copied()
            .filter(|&idx| remaining_predecessors[idx] == 0)
            .collect::<Vec<_>>();
        let mut dispatch_order = Vec::with_capacity(unassigned.len());

        while !ready.is_empty() {
            let best_ready_pos = ready
                .iter()
                .enumerate()
                .min_by_key(|&(_, &idx)| {
                    let owner_idx = owners[idx];
                    let start = predecessor_ready[idx].max(owner_ready[owner_idx]);
                    let finish = start.saturating_add(durations[idx]);
                    let order_key = order_key_fn
                        .map(|key_fn| key_fn(solution, unassigned[idx]))
                        .unwrap_or(0);
                    (start, Reverse(downstream[idx]), order_key, finish, idx)
                })
                .map(|(pos, _)| pos)?;
            let idx = ready.remove(best_ready_pos);
            let owner_idx = owners[idx];
            let start = predecessor_ready[idx].max(owner_ready[owner_idx]);
            let finish = start.saturating_add(durations[idx]);
            owner_ready[owner_idx] = finish;
            dispatch_order.push((unassigned[idx], owner_idx));

            for &successor in &successors[idx] {
                predecessor_ready[successor] = predecessor_ready[successor].max(finish);
                remaining_predecessors[successor] -= 1;
                if remaining_predecessors[successor] == 0 {
                    ready.push(successor);
                }
            }
        }

        (dispatch_order.len() == unassigned.len()).then_some(dispatch_order)
    }

    fn apply_owner_ordered_best_insertion<'t, 'a, D, BestCb>(
        &self,
        phase_scope: &mut PhaseScope<'t, 'a, S, D, BestCb>,
        owned_order: &[(E, usize)],
    ) where
        D: Director<S>,
        BestCb: crate::scope::ProgressCallback<S>,
    {
        for &(element, owner_idx) in owned_order {
            if phase_scope
                .solver_scope_mut()
                .should_interrupt_mandatory_construction()
            {
                break;
            }

            let solution = phase_scope.score_director().working_solution();
            let mut best: Option<(usize, S::Score)> = None;
            let len = (self.state.list_len)(solution, owner_idx);
            for pos in 0..=len {
                if phase_scope
                    .solver_scope_mut()
                    .should_interrupt_mandatory_construction()
                {
                    break;
                }
                if let Some(score) = self.state.eval_insertion(
                    element,
                    owner_idx,
                    pos,
                    phase_scope.score_director_mut(),
                ) {
                    if best.is_none_or(|(_, best_score)| score > best_score) {
                        best = Some((pos, score));
                    }
                }
            }
            let Some((pos, score)) = best else {
                tracing::warn!(
                    "No valid owner-restricted insertion found for regret fallback element"
                );
                continue;
            };

            let mut step_scope = StepScope::new_with_control_policy(
                phase_scope,
                StepControlPolicy::CompleteMandatoryConstruction,
            );

            step_scope.apply_committed_change(|sd| {
                self.state.apply_insertion(element, owner_idx, pos, sd);
            });

            step_scope.set_step_score(score);
            step_scope.complete();
        }
    }

    fn solve_owner_ordered_best_insertion<'t, 'a, D, BestCb>(
        &self,
        phase_scope: &mut PhaseScope<'t, 'a, S, D, BestCb>,
        unassigned: &[E],
        n_entities: usize,
    ) where
        D: Director<S>,
        BestCb: crate::scope::ProgressCallback<S>,
    {
        let Some(owner_fn) = self.state.element_owner_fn else {
            return;
        };

        let mut owned_order = Vec::with_capacity(unassigned.len());
        for &element in unassigned {
            let solution = phase_scope.score_director().working_solution();
            let Some(owner_idx) = owner_fn(solution, &element).filter(|owner| *owner < n_entities)
            else {
                tracing::warn!("No valid owner found for owner-restricted regret fallback element");
                continue;
            };
            owned_order.push((element, owner_idx));
        }
        self.apply_owner_ordered_best_insertion(phase_scope, &owned_order);
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: Director<S>,
    BestCb: crate::scope::ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let n_elements =
            (self.state.element_count)(phase_scope.score_director().working_solution());
        let n_entities = (self.state.entity_count)(phase_scope.score_director().working_solution());

        if n_entities == 0 || n_elements == 0 {
            phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned: Vec<E> =
            (self.state.get_assigned)(phase_scope.score_director().working_solution());
        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping regret insertion");
            phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();
        let mut unassigned = self.state.unassigned_elements(
            phase_scope.score_director().working_solution(),
            n_elements,
            &assigned_set,
        );
        let downstream_by_element = self.precedence_downstream_by_element(
            phase_scope.score_director().working_solution(),
            &unassigned,
        );

        if self.solve_oversized_owner_restricted(&mut phase_scope, &unassigned, n_entities) {
            phase_scope.update_best_solution();
            return;
        }

        while !unassigned.is_empty() {
            if phase_scope
                .solver_scope_mut()
                .should_interrupt_mandatory_construction()
            {
                break;
            }

            let mut best_choice: Option<(
                RegretValue<S::Score>,
                usize,
                usize,
                usize,
                S::Score,
                usize,
            )> = None;
            let mut interrupted = false;

            for (list_idx, &element) in unassigned.iter().enumerate() {
                if phase_scope
                    .solver_scope_mut()
                    .should_interrupt_mandatory_construction()
                {
                    interrupted = true;
                    break;
                }
                if let Some((regret, entity_idx, pos, score)) =
                    self.evaluate_regret(element, n_entities, phase_scope.score_director_mut())
                {
                    let downstream = downstream_by_element
                        .as_ref()
                        .and_then(|values| values.get(&element))
                        .copied()
                        .unwrap_or(0);
                    let is_better = match &best_choice {
                        None => true,
                        Some((best_regret, _, _, _, best_score, best_downstream)) => {
                            regret_choice_is_better_with_downstream(
                                regret,
                                score,
                                downstream,
                                *best_regret,
                                *best_score,
                                *best_downstream,
                            )
                        }
                    };
                    if is_better {
                        best_choice = Some((regret, list_idx, entity_idx, pos, score, downstream));
                    }
                }
            }

            if interrupted {
                break;
            }

            match best_choice {
                None => {
                    tracing::warn!("No valid insertion found for remaining elements, stopping");
                    break;
                }
                Some((_regret, list_idx, entity_idx, pos, score, _downstream)) => {
                    let element = unassigned.remove(list_idx);

                    let mut step_scope = StepScope::new_with_control_policy(
                        &mut phase_scope,
                        StepControlPolicy::CompleteMandatoryConstruction,
                    );

                    step_scope.apply_committed_change(|sd| {
                        self.state.apply_insertion(element, entity_idx, pos, sd);
                    });

                    step_scope.set_step_score(score);
                    step_scope.complete();
                }
            }
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListRegretInsertion"
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
    use solverforge_core::score::HardSoftScore;
    use solverforge_scoring::{ConstraintMetadata, Director};

    use super::*;

    #[derive(Clone, Debug)]
    struct RegretPlan {
        routes: Vec<Vec<usize>>,
        score: Option<HardSoftScore>,
    }

    impl PlanningSolution for RegretPlan {
        type Score = HardSoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    struct RegretDirector {
        working_solution: RegretPlan,
        descriptor: SolutionDescriptor,
        score_fn: fn(&RegretPlan) -> HardSoftScore,
    }

    impl RegretDirector {
        fn new(solution: RegretPlan) -> Self {
            Self::with_score_fn(solution, singleton_regret_score)
        }

        fn with_score_fn(solution: RegretPlan, score_fn: fn(&RegretPlan) -> HardSoftScore) -> Self {
            Self {
                working_solution: solution,
                descriptor: SolutionDescriptor::new("RegretPlan", TypeId::of::<RegretPlan>()),
                score_fn,
            }
        }
    }

    impl Director<RegretPlan> for RegretDirector {
        fn working_solution(&self) -> &RegretPlan {
            &self.working_solution
        }

        fn working_solution_mut(&mut self) -> &mut RegretPlan {
            &mut self.working_solution
        }

        fn calculate_score(&mut self) -> HardSoftScore {
            let score = (self.score_fn)(&self.working_solution);
            self.working_solution.set_score(Some(score));
            score
        }

        fn solution_descriptor(&self) -> &SolutionDescriptor {
            &self.descriptor
        }

        fn clone_working_solution(&self) -> RegretPlan {
            self.working_solution.clone()
        }

        fn before_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

        fn after_variable_changed(&mut self, _descriptor_index: usize, _entity_index: usize) {}

        fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
            (descriptor_index == 0).then_some(self.working_solution.routes.len())
        }

        fn total_entity_count(&self) -> Option<usize> {
            Some(self.working_solution.routes.len())
        }

        fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
            Vec::new()
        }
    }

    fn singleton_regret_score(solution: &RegretPlan) -> HardSoftScore {
        match singleton_assignment(solution) {
            Some((0, 0)) => HardSoftScore::of(0, -2_000_000),
            Some((1, 0)) => HardSoftScore::of(-1, 0),
            Some((0, 1)) => HardSoftScore::of(0, 10),
            Some((1, 1)) => HardSoftScore::of(0, 0),
            _ => HardSoftScore::of(-10, 0),
        }
    }

    fn stable_order_score(solution: &RegretPlan) -> HardSoftScore {
        match solution.routes.first().map(Vec::as_slice) {
            Some([0]) => HardSoftScore::of(0, 10),
            Some([_]) => HardSoftScore::ZERO,
            _ => HardSoftScore::ZERO,
        }
    }

    fn singleton_assignment(solution: &RegretPlan) -> Option<(usize, usize)> {
        let mut assignment = None;
        for (entity_idx, route) in solution.routes.iter().enumerate() {
            for &element in route {
                if assignment.is_some() {
                    return None;
                }
                assignment = Some((entity_idx, element));
            }
        }
        assignment
    }

    fn element_count(_: &RegretPlan) -> usize {
        2
    }

    fn four_element_count(_: &RegretPlan) -> usize {
        4
    }

    fn twenty_element_count(_: &RegretPlan) -> usize {
        20
    }

    fn large_owner_restricted_element_count(_: &RegretPlan) -> usize {
        47
    }

    fn very_large_owner_restricted_element_count(_: &RegretPlan) -> usize {
        300
    }

    fn very_large_two_owner_precedence_element_count(_: &RegretPlan) -> usize {
        400
    }

    fn get_assigned(solution: &RegretPlan) -> Vec<usize> {
        solution
            .routes
            .iter()
            .flat_map(|route| route.iter().copied())
            .collect()
    }

    fn entity_count(solution: &RegretPlan) -> usize {
        solution.routes.len()
    }

    fn list_len(solution: &RegretPlan, entity_idx: usize) -> usize {
        solution.routes[entity_idx].len()
    }

    fn list_insert(solution: &mut RegretPlan, entity_idx: usize, pos: usize, element: usize) {
        solution.routes[entity_idx].insert(pos, element);
    }

    fn list_remove(solution: &mut RegretPlan, entity_idx: usize, pos: usize) -> usize {
        solution.routes[entity_idx].remove(pos)
    }

    fn index_to_element(_: &RegretPlan, idx: usize) -> usize {
        idx
    }

    fn swapped_index_to_element(_: &RegretPlan, idx: usize) -> usize {
        match idx {
            0 => 1,
            1 => 0,
            _ => idx,
        }
    }

    fn single_owner(_: &RegretPlan, _: &usize) -> Option<usize> {
        Some(0)
    }

    fn same_owner(_: &RegretPlan, _: &usize) -> Option<usize> {
        Some(0)
    }

    fn two_owner_by_parity(_: &RegretPlan, element: &usize) -> Option<usize> {
        Some(element % 2)
    }

    fn dispatch_start_owner(_: &RegretPlan, element: &usize) -> Option<usize> {
        match element {
            0 => Some(2),
            1 => Some(1),
            2 => Some(0),
            3 => Some(1),
            _ => None,
        }
    }

    fn unit_duration(_: &RegretPlan, _: usize) -> usize {
        1
    }

    fn dispatch_start_duration(_: &RegretPlan, element: usize) -> usize {
        match element {
            0 => 100,
            1 => 50,
            _ => 1,
        }
    }

    fn descending_same_owner_successor(_: &RegretPlan, element: usize, out: &mut Vec<usize>) {
        if element >= 2 {
            out.push(element - 2);
        }
    }

    fn dispatch_start_successor(_: &RegretPlan, element: usize, out: &mut Vec<usize>) {
        match element {
            0 => out.push(2),
            1 => out.push(3),
            _ => {}
        }
    }

    fn dispatch_conflicting_order_key(_: &RegretPlan, element: usize) -> i64 {
        match element {
            0 => 10,
            1 => 0,
            _ => 0,
        }
    }

    fn two_chain_successor(_: &RegretPlan, element: usize, out: &mut Vec<usize>) {
        match element {
            0 => out.push(2),
            1 => out.push(3),
            _ => {}
        }
    }

    fn long_chain_successor(_: &RegretPlan, element: usize, out: &mut Vec<usize>) {
        match element {
            0 => out.push(2),
            2 => out.push(3),
            _ => {}
        }
    }

    fn no_successor(_: &RegretPlan, _: usize, _: &mut Vec<usize>) {}

    fn descending_weight_score(solution: &RegretPlan) -> HardSoftScore {
        let weighted = solution
            .routes
            .iter()
            .map(|route| {
                let len = route.len();
                route
                    .iter()
                    .enumerate()
                    .map(|(pos, &element)| (len - pos) as i64 * element as i64)
                    .sum::<i64>()
            })
            .sum();
        HardSoftScore::of(0, weighted)
    }

    fn prefer_one_first_score(solution: &RegretPlan) -> HardSoftScore {
        match solution.routes.first().and_then(|route| route.first()) {
            Some(1) => HardSoftScore::of(0, 10),
            Some(_) => HardSoftScore::ZERO,
            None => HardSoftScore::ZERO,
        }
    }

    fn zero_score(_: &RegretPlan) -> HardSoftScore {
        HardSoftScore::ZERO
    }

    #[test]
    fn regret_compares_score_levels_not_scalar_projection() {
        let mut director = RegretDirector::new(RegretPlan {
            routes: vec![Vec::new(), Vec::new()],
            score: None,
        });
        let phase = ListRegretInsertionPhase::new(
            element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        );

        let (hard_regret, hard_entity, hard_pos, _) = phase
            .evaluate_regret(0, 2, &mut director)
            .expect("hard regret");
        let (soft_regret, soft_entity, soft_pos, _) = phase
            .evaluate_regret(1, 2, &mut director)
            .expect("soft regret");

        assert_eq!((hard_entity, hard_pos), (0, 0));
        assert_eq!((soft_entity, soft_pos), (0, 0));
        assert!(
            hard_regret > soft_regret,
            "a hard-level regret must outrank a larger soft-level regret"
        );
    }

    #[test]
    fn regret_ties_prefer_better_best_insertion_score() {
        let regret = RegretValue::Finite(HardSoftScore::of(0, -1));

        assert!(regret_choice_is_better(
            regret,
            HardSoftScore::of(0, -5),
            regret,
            HardSoftScore::of(0, -6),
        ));
        assert!(!regret_choice_is_better(
            regret,
            HardSoftScore::of(0, -7),
            regret,
            HardSoftScore::of(0, -6),
        ));
        assert!(regret_choice_is_better(
            RegretValue::Forced,
            HardSoftScore::of(0, -100),
            regret,
            HardSoftScore::of(0, -1),
        ));
    }

    #[test]
    fn regret_ties_prefer_precedence_downstream_after_score() {
        let regret = RegretValue::Finite(HardSoftScore::ZERO);

        assert!(regret_choice_is_better_with_downstream(
            regret,
            HardSoftScore::ZERO,
            10,
            regret,
            HardSoftScore::ZERO,
            5,
        ));
        assert!(!regret_choice_is_better_with_downstream(
            regret,
            HardSoftScore::ZERO,
            5,
            regret,
            HardSoftScore::ZERO,
            10,
        ));
        assert!(regret_choice_is_better_with_downstream(
            regret,
            HardSoftScore::of(0, 1),
            1,
            regret,
            HardSoftScore::ZERO,
            10,
        ));
    }

    #[test]
    fn precedence_frontier_choice_prefers_downstream_before_regret_score() {
        let low_regret = RegretValue::Finite(HardSoftScore::of(0, -1));
        let high_regret = RegretValue::Finite(HardSoftScore::of(0, 100));

        assert!(precedence_frontier_choice_is_better(
            20,
            low_regret,
            HardSoftScore::of(0, -100),
            10,
            high_regret,
            HardSoftScore::of(0, 100),
        ));
        assert!(!precedence_frontier_choice_is_better(
            10,
            high_regret,
            HardSoftScore::of(0, 100),
            20,
            low_regret,
            HardSoftScore::of(0, -100),
        ));
        assert!(precedence_frontier_choice_is_better(
            20,
            high_regret,
            HardSoftScore::of(0, 100),
            20,
            low_regret,
            HardSoftScore::of(0, -100),
        ));
    }

    #[test]
    fn owner_restricted_trial_budget_keeps_large_fixed_owner_construction_bounded() {
        assert!(
            owner_restricted_regret_trial_count(&vec![20; 20])
                > OWNER_RESTRICTED_REGRET_TRIAL_BUDGET
        );
        assert!(
            owner_restricted_regret_trial_count(&vec![5; 10])
                <= OWNER_RESTRICTED_REGRET_TRIAL_BUDGET
        );
        assert!(
            owner_restricted_best_insertion_trial_count(&[47]) * 47
                <= OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET
        );
        assert!(
            owner_restricted_best_insertion_trial_count(&[300]) * 300
                > OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET
        );
    }

    #[test]
    fn oversized_owner_restricted_construction_uses_best_insertion_not_append() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            descending_weight_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListRegretInsertionPhase::new(
            large_owner_restricted_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(single_owner));

        phase.solve(&mut solver_scope);

        let route = &solver_scope.working_solution().routes[0];
        assert_eq!(route.len(), 47);
        assert_eq!(route.first(), Some(&46));
        assert_eq!(route.last(), Some(&0));
    }

    #[test]
    fn very_large_owner_restricted_construction_keeps_append_fallback() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            descending_weight_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListRegretInsertionPhase::new(
            very_large_owner_restricted_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(single_owner));

        phase.solve(&mut solver_scope);

        let route = &solver_scope.working_solution().routes[0];
        assert_eq!(route.len(), 300);
        assert_eq!(route.first(), Some(&0));
        assert_eq!(route.last(), Some(&299));
    }

    #[test]
    fn oversized_owner_restricted_precedence_uses_frontier_before_dispatch_append() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            descending_weight_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListRegretInsertionPhase::new(
            large_owner_restricted_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(single_owner))
        .with_precedence_hooks(Some(unit_duration), Some(no_successor));

        phase.solve(&mut solver_scope);

        let route = &solver_scope.working_solution().routes[0];
        assert_eq!(route.len(), 47);
        assert_eq!(route.first(), Some(&46));
        assert_eq!(route.last(), Some(&0));
    }

    #[test]
    fn very_large_owner_restricted_precedence_construction_stays_bounded() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new(), Vec::new()],
                score: None,
            },
            descending_weight_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListRegretInsertionPhase::new(
            very_large_two_owner_precedence_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(two_owner_by_parity))
        .with_precedence_hooks(Some(unit_duration), Some(descending_same_owner_successor));

        phase.solve(&mut solver_scope);

        let routes = &solver_scope.working_solution().routes;
        assert_eq!(routes[0].len(), 200);
        assert_eq!(routes[1].len(), 200);
        assert_eq!(routes[0].first(), Some(&398));
        assert_eq!(routes[0].last(), Some(&0));
        assert_eq!(routes[1].first(), Some(&399));
        assert_eq!(routes[1].last(), Some(&1));
    }

    #[test]
    fn precedence_frontier_regret_selects_best_ready_element() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            prefer_one_first_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let phase = ListRegretInsertionPhase::new(
            four_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(same_owner))
        .with_precedence_hooks(Some(unit_duration), Some(two_chain_successor));

        {
            let mut phase_scope = PhaseScope::new(&mut solver_scope, 0);
            assert!(phase.solve_precedence_frontier_regret(&mut phase_scope, &[0, 1, 2, 3], 1));
        }

        assert_eq!(solver_scope.working_solution().routes[0].first(), Some(&1));
        assert_eq!(solver_scope.working_solution().routes[0].len(), 4);
    }

    #[test]
    fn full_regret_uses_precedence_downstream_only_as_tie_breaker() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            zero_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListRegretInsertionPhase::new(
            four_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            swapped_index_to_element,
            0,
        )
        .with_precedence_hooks(Some(unit_duration), Some(long_chain_successor));

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes[0].last(),
            Some(&0),
            "element 0 has the longest downstream chain and should win a regret/score tie"
        );
    }

    #[test]
    fn precedence_frontier_regret_is_budget_gated_not_width_gated() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            descending_weight_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let phase = ListRegretInsertionPhase::new(
            twenty_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(same_owner))
        .with_precedence_hooks(Some(unit_duration), Some(no_successor));
        let unassigned = (0..20).collect::<Vec<_>>();

        {
            let mut phase_scope = PhaseScope::new(&mut solver_scope, 0);
            assert!(phase.solve_precedence_frontier_regret(&mut phase_scope, &unassigned, 1));
        }

        assert_eq!(solver_scope.working_solution().routes[0].len(), 20);
        assert_eq!(solver_scope.working_solution().routes[0].first(), Some(&19));
    }

    #[test]
    fn precedence_dispatch_prefers_earliest_feasible_start_before_idle_owner() {
        let plan = RegretPlan {
            routes: vec![Vec::new(), Vec::new(), Vec::new()],
            score: None,
        };
        let phase = ListRegretInsertionPhase::new(
            four_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(dispatch_start_owner))
        .with_precedence_hooks(
            Some(dispatch_start_duration),
            Some(dispatch_start_successor),
        );

        let order = phase
            .precedence_dispatch_order(&plan, &[0, 1, 2, 3], 3)
            .expect("dispatch order");

        assert_eq!(
            order
                .into_iter()
                .map(|(element, _)| element)
                .collect::<Vec<_>>(),
            vec![0, 1, 3, 2]
        );
    }

    #[test]
    fn precedence_dispatch_prefers_critical_downstream_before_order_key() {
        let plan = RegretPlan {
            routes: vec![Vec::new(), Vec::new(), Vec::new()],
            score: None,
        };
        let phase = ListRegretInsertionPhase::new(
            four_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        )
        .with_element_owner_fn(Some(dispatch_start_owner))
        .with_element_order_key(Some(dispatch_conflicting_order_key))
        .with_precedence_hooks(
            Some(dispatch_start_duration),
            Some(dispatch_start_successor),
        );

        let order = phase
            .precedence_dispatch_order(&plan, &[0, 1, 2, 3], 3)
            .expect("dispatch order");

        assert_eq!(
            order
                .into_iter()
                .map(|(element, _)| element)
                .collect::<Vec<_>>(),
            vec![0, 1, 3, 2]
        );
    }

    #[test]
    fn regret_removal_preserves_remaining_unassigned_order() {
        let director = RegretDirector::with_score_fn(
            RegretPlan {
                routes: vec![Vec::new()],
                score: None,
            },
            stable_order_score,
        );
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListRegretInsertionPhase::new(
            four_element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            index_to_element,
            0,
        );

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes,
            vec![vec![3, 2, 1, 0]]
        );
    }
}
