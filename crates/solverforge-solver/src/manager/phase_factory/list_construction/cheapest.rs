use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::state::ScoredConstructionState;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepScope};

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
/// let phase = ListCheapestInsertionPhase::<Plan, usize>::new(
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
    /* Creates a new cheapest insertion phase.

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
                descriptor_index,
            },
            _marker: PhantomData,
        }
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListCheapestInsertionPhase<S, E>
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
            tracing::info!("All elements already assigned, skipping construction");
            let _score = phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        for elem_idx in 0..n_elements {
            if phase_scope
                .solver_scope_mut()
                .should_terminate_construction()
            {
                break;
            }

            let element = (self.state.index_to_element)(
                phase_scope.score_director().working_solution(),
                elem_idx,
            );
            if assigned_set.contains(&element) {
                continue;
            }

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
