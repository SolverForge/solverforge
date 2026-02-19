//! List construction phase for assigning list elements to entities.
//!
//! Provides several construction strategies for list variables
//! (e.g., assigning visits to vehicles in VRP):
//!
//! - [`ListConstructionPhase`]: Simple round-robin assignment
//! - [`ListCheapestInsertionPhase`]: Score-guided greedy insertion
//! - [`ListRegretInsertionPhase`]: Regret-based insertion (reduces greedy myopia)

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};

use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

use super::super::PhaseFactory;

/// Builder for creating list construction phases.
///
/// This builder creates phases that assign unassigned list elements to entities
/// using a round-robin strategy. Ideal for VRP-style problems where visits
/// need to be distributed across vehicles.
///
/// # Type Parameters
///
/// * `S` - The planning solution type
/// * `E` - The element type (e.g., visit index)
///
/// # Example
///
/// ```
/// use solverforge_solver::{ListConstructionPhase, ListConstructionPhaseBuilder};
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan { vehicles: Vec<Vehicle>, visits: Vec<()>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Plan {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let builder = ListConstructionPhaseBuilder::<Plan, usize>::new(
///     |plan| plan.visits.len(),
///     |plan| plan.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect(),
///     |plan| plan.vehicles.len(),
///     |plan, entity_idx, element| { plan.vehicles[entity_idx].visits.push(element); },
///     |idx| idx,
///     "visits",
///     1,
/// );
///
/// // Create a concrete phase:
/// let phase: ListConstructionPhase<Plan, usize> = builder.create_phase();
/// ```
pub struct ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(usize) -> E,
    variable_name: &'static str,
    descriptor_index: usize,
    _marker: PhantomData<(fn() -> S, fn() -> E)>,
}

impl<S, E> ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    /// Creates a new list construction phase builder.
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        assign_element: fn(&mut S, usize, E),
        index_to_element: fn(usize) -> E,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            assign_element,
            index_to_element,
            variable_name,
            descriptor_index,
            _marker: PhantomData,
        }
    }

    /// Creates the list construction phase.
    pub fn create_phase(&self) -> ListConstructionPhase<S, E> {
        ListConstructionPhase {
            element_count: self.element_count,
            get_assigned: self.get_assigned,
            entity_count: self.entity_count,
            assign_element: self.assign_element,
            index_to_element: self.index_to_element,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            _marker: PhantomData,
        }
    }
}

impl<S, E, D> PhaseFactory<S, D> for ListConstructionPhaseBuilder<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: ScoreDirector<S>,
{
    type Phase = ListConstructionPhase<S, E>;

    fn create(&self) -> Self::Phase {
        ListConstructionPhaseBuilder::create_phase(self)
    }
}

/// List construction phase that assigns elements round-robin to entities.
pub struct ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(usize) -> E,
    variable_name: &'static str,
    descriptor_index: usize,
    _marker: PhantomData<(fn() -> S, fn() -> E)>,
}

impl<S, E> std::fmt::Debug for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListConstructionPhase").finish()
    }
}

impl<S, E, D> Phase<S, D> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        let mut phase_scope = PhaseScope::new(solver_scope, 0);

        let solution = phase_scope.score_director().working_solution();
        let n_elements = (self.element_count)(solution);
        let n_entities = (self.entity_count)(solution);

        if n_entities == 0 || n_elements == 0 {
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned: Vec<E> = (self.get_assigned)(phase_scope.score_director().working_solution());

        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping construction");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        let mut entity_idx = 0;
        for elem_idx in 0..n_elements {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            let element = (self.index_to_element)(elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            let mut step_scope = StepScope::new(&mut phase_scope);

            {
                let sd = step_scope.score_director_mut();
                sd.before_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
                (self.assign_element)(sd.working_solution_mut(), entity_idx, element);
                sd.after_variable_changed(self.descriptor_index, entity_idx, self.variable_name);
            }

            let step_score = step_scope.calculate_score();
            step_scope.set_step_score(step_score);
            step_scope.complete();

            entity_idx = (entity_idx + 1) % n_entities;
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}

// ---------------------------------------------------------------------------
// Shared utilities for scored construction phases
// ---------------------------------------------------------------------------

/// Common fields for scored list construction phases.
///
/// Holds all function pointers needed to enumerate elements, enumerate entities,
/// try insertions, and apply the chosen insertion.
struct ScoredConstructionState<S, E> {
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_insert: fn(&mut S, usize, usize, E),
    list_remove: fn(&mut S, usize, usize) -> E,
    index_to_element: fn(usize) -> E,
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, E> ScoredConstructionState<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    /// Evaluate the score delta of inserting `element` at `(entity_idx, pos)`.
    ///
    /// Performs: before_changed → insert → score → remove → after_changed (undo).
    /// Returns the score if the insertion is evaluable.
    fn eval_insertion<D: ScoreDirector<S>>(
        &self,
        element: E,
        entity_idx: usize,
        pos: usize,
        score_director: &mut D,
    ) -> Option<S::Score> {
        let list_insert = self.list_insert;
        let list_remove = self.list_remove;
        let descriptor_index = self.descriptor_index;
        let variable_name = self.variable_name;

        let mut recording = RecordingScoreDirector::new(score_director);

        // Before change notification
        recording.before_variable_changed(descriptor_index, entity_idx, variable_name);

        // Insert element
        list_insert(recording.working_solution_mut(), entity_idx, pos, element);

        // After change notification
        recording.after_variable_changed(descriptor_index, entity_idx, variable_name);

        // Register undo closure
        recording.register_undo(Box::new(move |s: &mut S| {
            list_remove(s, entity_idx, pos);
        }));

        // Evaluate score
        let score = recording.calculate_score();

        // Undo
        recording.undo_changes();

        Some(score)
    }

    /// Find the best (entity_idx, pos, score) for inserting `element`.
    ///
    /// Returns `None` if there are no valid insertion points.
    fn best_insertion<D: ScoreDirector<S>>(
        &self,
        element: E,
        n_entities: usize,
        score_director: &mut D,
    ) -> Option<(usize, usize, S::Score)> {
        let list_len = self.list_len;
        let mut best: Option<(usize, usize, S::Score)> = None;

        for entity_idx in 0..n_entities {
            let len = list_len(score_director.working_solution(), entity_idx);
            // Insert at any position 0..=len
            for pos in 0..=len {
                if let Some(score) = self.eval_insertion(element, entity_idx, pos, score_director) {
                    let is_better = match &best {
                        None => true,
                        Some((_, _, best_score)) => score > *best_score,
                    };
                    if is_better {
                        best = Some((entity_idx, pos, score));
                    }
                }
            }
        }

        best
    }

    /// Apply the best insertion for `element` permanently.
    ///
    /// Returns true if the element was inserted.
    fn apply_insertion<D: ScoreDirector<S>>(
        &self,
        element: E,
        entity_idx: usize,
        pos: usize,
        score_director: &mut D,
    ) {
        score_director.before_variable_changed(
            self.descriptor_index,
            entity_idx,
            self.variable_name,
        );
        (self.list_insert)(
            score_director.working_solution_mut(),
            entity_idx,
            pos,
            element,
        );
        score_director.after_variable_changed(
            self.descriptor_index,
            entity_idx,
            self.variable_name,
        );
    }
}

// ---------------------------------------------------------------------------
// ListCheapestInsertionPhase
// ---------------------------------------------------------------------------

/// List construction phase using greedy cheapest insertion.
///
/// For each unassigned element (in index order), evaluates all possible insertion
/// positions across all entities and inserts it at the position that yields the
/// best score. This is significantly better than round-robin for VRP because:
///
/// - Routes are built incrementally with score feedback
/// - Capacity constraints influence which vehicle receives each visit
/// - Distance/duration constraints are optimized at the point of insertion
///
/// # Algorithm
///
/// ```text
/// for each unassigned element e:
///     for each entity r and each position p in r:
///         try insert(e, r, p), score, undo
///     permanently insert e at (r*, p*) with best score
/// ```
///
/// Complexity: O(E × N × M) where E = elements, N = entities, M = avg route length.
///
/// # Example
///
/// ```
/// use solverforge_solver::ListCheapestInsertionPhase;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan { vehicles: Vec<Vehicle>, n_visits: usize, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Plan {
///     type Score = SimpleScore;
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
/// let phase = ListCheapestInsertionPhase::<Plan, usize>::new(
///     |p| p.n_visits,
///     |p| p.vehicles.iter().flat_map(|v| v.visits.iter().copied()).collect(),
///     |p| p.vehicles.len(),
///     list_len,
///     list_insert,
///     list_remove,
///     |idx| idx,
///     "visits",
///     0,
/// );
/// ```
pub struct ListCheapestInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    state: ScoredConstructionState<S, E>,
    _marker: PhantomData<fn() -> (S, E)>,
}

impl<S, E> std::fmt::Debug for ListCheapestInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListCheapestInsertionPhase").finish()
    }
}

impl<S, E> ListCheapestInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
{
    /// Creates a new cheapest insertion phase.
    ///
    /// # Arguments
    /// * `element_count` - Total number of elements to assign
    /// * `get_assigned` - Returns currently assigned elements
    /// * `entity_count` - Total number of entities (routes/vehicles)
    /// * `list_len` - Length of entity's list
    /// * `list_insert` - Insert element at position (shifts right)
    /// * `list_remove` - Remove element at position (used for undo), returns removed element
    /// * `index_to_element` - Converts element index to element value
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
        list_remove: fn(&mut S, usize, usize) -> E,
        index_to_element: fn(usize) -> E,
        variable_name: &'static str,
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
                variable_name,
                descriptor_index,
            },
            _marker: PhantomData,
        }
    }
}

impl<S, E, D> Phase<S, D> for ListCheapestInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
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
            tracing::info!("All elements already assigned, skipping construction");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        for elem_idx in 0..n_elements {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            let element = (self.state.index_to_element)(elem_idx);
            if assigned_set.contains(&element) {
                continue;
            }

            // Find best insertion position
            let best =
                self.state
                    .best_insertion(element, n_entities, phase_scope.score_director_mut());

            if let Some((entity_idx, pos, _score)) = best {
                let mut step_scope = StepScope::new(&mut phase_scope);

                self.state.apply_insertion(
                    element,
                    entity_idx,
                    pos,
                    step_scope.score_director_mut(),
                );

                let step_score = step_scope.calculate_score();
                step_scope.set_step_score(step_score);
                step_scope.complete();
            } else {
                tracing::warn!("No valid insertion found for element {:?}", elem_idx);
            }
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListCheapestInsertion"
    }
}

// ---------------------------------------------------------------------------
// ListRegretInsertionPhase
// ---------------------------------------------------------------------------

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
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Vehicle { visits: Vec<usize> }
///
/// #[derive(Clone)]
/// struct Plan { vehicles: Vec<Vehicle>, n_visits: usize, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Plan {
///     type Score = SimpleScore;
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
///     |idx| idx,
///     "visits",
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
    /// Creates a new regret insertion phase.
    ///
    /// # Arguments
    /// * `element_count` - Total number of elements to assign
    /// * `get_assigned` - Returns currently assigned elements
    /// * `entity_count` - Total number of entities (routes/vehicles)
    /// * `list_len` - Length of entity's list
    /// * `list_insert` - Insert element at position (shifts right)
    /// * `list_remove` - Remove element at position (used for undo), returns removed element
    /// * `index_to_element` - Converts element index to element value
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_insert: fn(&mut S, usize, usize, E),
        list_remove: fn(&mut S, usize, usize) -> E,
        index_to_element: fn(usize) -> E,
        variable_name: &'static str,
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
                variable_name,
                descriptor_index,
            },
            _marker: PhantomData,
        }
    }

    /// Evaluate best and second-best insertions for `element`.
    ///
    /// Returns `(regret, best_entity, best_pos)` where regret is the score
    /// difference between first and second best insertion.
    fn evaluate_regret<D: ScoreDirector<S>>(
        &self,
        element: E,
        n_entities: usize,
        score_director: &mut D,
    ) -> Option<(f64, usize, usize)> {
        let list_len = self.state.list_len;
        let mut all_insertions: Vec<(usize, usize, S::Score)> = Vec::new();

        for entity_idx in 0..n_entities {
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

        // Sort descending by score (best first)
        all_insertions.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let (best_entity, best_pos, best_score) = all_insertions[0];

        // Regret = score improvement vs second best (0 if only one option)
        let regret = if all_insertions.len() >= 2 {
            let second_score = all_insertions[1].2;
            // Convert to f64 via f64 comparison — higher score = better
            // Use the hard/soft structure: positive regret means best is uniquely better
            if best_score > second_score {
                1.0 // Has unique best — prioritize this element
            } else {
                0.0 // Tied — lower priority
            }
        } else {
            2.0 // Only one option — highest regret (must place here or nowhere)
        };

        Some((regret, best_entity, best_pos))
    }
}

impl<S, E, D> Phase<S, D> for ListRegretInsertionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + PartialEq + Eq + std::hash::Hash + Send + Sync + 'static,
    D: ScoreDirector<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
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

        // Build list of unassigned elements
        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();
        let mut unassigned: Vec<E> = (0..n_elements)
            .map(|i| (self.state.index_to_element)(i))
            .filter(|e| !assigned_set.contains(e))
            .collect();

        while !unassigned.is_empty() {
            if phase_scope.solver_scope().should_terminate() {
                break;
            }

            // For each unassigned element, compute regret and best insertion
            let mut best_choice: Option<(f64, usize, usize, usize)> = None;
            // (regret, elem_list_idx, entity_idx, pos)

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

                    self.state.apply_insertion(
                        element,
                        entity_idx,
                        pos,
                        step_scope.score_director_mut(),
                    );

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
