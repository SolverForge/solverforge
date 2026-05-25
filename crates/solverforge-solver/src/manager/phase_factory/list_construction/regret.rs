use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::state::ScoredConstructionState;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

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

    /* Evaluate best and second-best insertions for `element`. */
    fn evaluate_regret<D: Director<S>>(
        &self,
        element: E,
        n_entities: usize,
        score_director: &mut D,
    ) -> Option<(RegretValue<S::Score>, usize, usize)> {
        let list_len = self.state.list_len;
        let mut all_insertions: Vec<(usize, usize, S::Score)> = Vec::new();

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
                    all_insertions.push((entity_idx, pos, score));
                }
            }
        }

        if all_insertions.is_empty() {
            return None;
        }

        all_insertions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let (best_entity, best_pos, best_score) = all_insertions[0];

        let regret = if all_insertions.len() >= 2 {
            let second_score = all_insertions[1].2;
            RegretValue::Finite(best_score - second_score)
        } else {
            RegretValue::Forced
        };

        Some((regret, best_entity, best_pos))
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
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned: Vec<E> =
            (self.state.get_assigned)(phase_scope.score_director().working_solution());
        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping regret insertion");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();
        let solution = phase_scope.score_director().working_solution();
        let mut unassigned: Vec<E> = (0..n_elements)
            .map(|i| (self.state.index_to_element)(solution, i))
            .filter(|e| !assigned_set.contains(e))
            .collect();

        while !unassigned.is_empty() {
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let mut best_choice: Option<(RegretValue<S::Score>, usize, usize, usize)> = None;

            for (list_idx, &element) in unassigned.iter().enumerate() {
                if let Some((regret, entity_idx, pos)) =
                    self.evaluate_regret(element, n_entities, phase_scope.score_director_mut())
                {
                    let is_better = match &best_choice {
                        None => true,
                        Some((best_regret, _, _, _)) => regret > *best_regret,
                    };
                    if is_better {
                        best_choice = Some((regret, list_idx, entity_idx, pos));
                    }
                }
            }

            match best_choice {
                None => {
                    tracing::warn!("No valid insertion found for remaining elements, stopping");
                    break;
                }
                Some((_regret, list_idx, entity_idx, pos)) => {
                    let element = unassigned.swap_remove(list_idx);

                    let mut step_scope = StepScope::new(&mut phase_scope);

                    step_scope.apply_committed_change(|sd| {
                        self.state.apply_insertion(element, entity_idx, pos, sd);
                    });

                    let step_score = step_scope.calculate_score();
                    step_scope.set_step_score(step_score);
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
    }

    impl RegretDirector {
        fn new(solution: RegretPlan) -> Self {
            Self {
                working_solution: solution,
                descriptor: SolutionDescriptor::new("RegretPlan", TypeId::of::<RegretPlan>()),
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
            let score = match singleton_assignment(&self.working_solution) {
                Some((0, 0)) => HardSoftScore::of(0, -2_000_000),
                Some((1, 0)) => HardSoftScore::of(-1, 0),
                Some((0, 1)) => HardSoftScore::of(0, 10),
                Some((1, 1)) => HardSoftScore::of(0, 0),
                _ => HardSoftScore::of(-10, 0),
            };
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

        let (hard_regret, hard_entity, hard_pos) = phase
            .evaluate_regret(0, 2, &mut director)
            .expect("hard regret");
        let (soft_regret, soft_entity, soft_pos) = phase
            .evaluate_regret(1, 2, &mut director)
            .expect("soft regret");

        assert_eq!((hard_entity, hard_pos), (0, 0));
        assert_eq!((soft_entity, soft_pos), (0, 0));
        assert!(
            hard_regret > soft_regret,
            "a hard-level regret must outrank a larger soft-level regret"
        );
    }
}
