use std::cmp::Reverse;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::state::ScoredConstructionState;
use crate::phase::Phase;
use crate::scope::{PhaseScope, SolverScope, StepControlPolicy, StepScope};

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

    pub fn with_precedence_hooks(
        mut self,
        duration_fn: Option<fn(&S, E) -> usize>,
        successors_fn: Option<fn(&S, E, &mut Vec<E>)>,
    ) -> Self {
        self.state.precedence_duration_fn = duration_fn;
        self.state.precedence_successors_fn = successors_fn;
        self
    }

    fn order_precedence_elements(&self, solution: &S, elements: &mut [E]) {
        let Some(downstream_by_element) = self
            .state
            .precedence_downstream_by_element(solution, elements)
        else {
            return;
        };
        elements.sort_by_key(|element| {
            Reverse(downstream_by_element.get(element).copied().unwrap_or(0))
        });
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
            phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned: Vec<E> =
            (self.state.get_assigned)(phase_scope.score_director().working_solution());
        if assigned.len() >= n_elements {
            tracing::info!("All elements already assigned, skipping construction");
            phase_scope.score_director_mut().calculate_score();
            phase_scope.update_best_solution();
            return;
        }

        let assigned_set: std::collections::HashSet<E> = assigned.into_iter().collect();

        let mut elements = self.state.unassigned_elements(
            phase_scope.score_director().working_solution(),
            n_elements,
            &assigned_set,
        );
        self.order_precedence_elements(
            phase_scope.score_director().working_solution(),
            &mut elements,
        );

        for element in elements {
            if phase_scope
                .solver_scope_mut()
                .should_interrupt_mandatory_construction()
            {
                break;
            }

            let best =
                self.state
                    .best_insertion(element, n_entities, phase_scope.score_director_mut());

            if let Some((entity_idx, pos, score)) = best {
                let mut step_scope = StepScope::new_with_control_policy(
                    &mut phase_scope,
                    StepControlPolicy::CompleteMandatoryConstruction,
                );

                step_scope.apply_committed_change(|sd| {
                    self.state.apply_insertion(element, entity_idx, pos, sd);
                });

                step_scope.set_step_score(score);
                step_scope.complete();
            } else {
                tracing::warn!("No valid insertion found for list element");
            }
        }

        phase_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "ListCheapestInsertion"
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
    struct CheapestPlan {
        routes: Vec<Vec<usize>>,
        score: Option<HardSoftScore>,
    }

    impl PlanningSolution for CheapestPlan {
        type Score = HardSoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    struct CheapestDirector {
        working_solution: CheapestPlan,
        descriptor: SolutionDescriptor,
    }

    impl CheapestDirector {
        fn new(solution: CheapestPlan) -> Self {
            Self {
                working_solution: solution,
                descriptor: SolutionDescriptor::new("CheapestPlan", TypeId::of::<CheapestPlan>()),
            }
        }
    }

    impl Director<CheapestPlan> for CheapestDirector {
        fn working_solution(&self) -> &CheapestPlan {
            &self.working_solution
        }

        fn working_solution_mut(&mut self) -> &mut CheapestPlan {
            &mut self.working_solution
        }

        fn calculate_score(&mut self) -> HardSoftScore {
            let score = HardSoftScore::ZERO;
            self.working_solution.set_score(Some(score));
            score
        }

        fn solution_descriptor(&self) -> &SolutionDescriptor {
            &self.descriptor
        }

        fn clone_working_solution(&self) -> CheapestPlan {
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

    fn element_count(_: &CheapestPlan) -> usize {
        2
    }

    fn get_assigned(solution: &CheapestPlan) -> Vec<usize> {
        solution
            .routes
            .iter()
            .flat_map(|route| route.iter().copied())
            .collect()
    }

    fn entity_count(solution: &CheapestPlan) -> usize {
        solution.routes.len()
    }

    fn list_len(solution: &CheapestPlan, entity_idx: usize) -> usize {
        solution.routes[entity_idx].len()
    }

    fn list_insert(solution: &mut CheapestPlan, entity_idx: usize, pos: usize, element: usize) {
        solution.routes[entity_idx].insert(pos, element);
    }

    fn list_remove(solution: &mut CheapestPlan, entity_idx: usize, pos: usize) -> usize {
        solution.routes[entity_idx].remove(pos)
    }

    fn swapped_index_to_element(_: &CheapestPlan, idx: usize) -> usize {
        match idx {
            0 => 1,
            1 => 0,
            _ => idx,
        }
    }

    fn unit_duration(_: &CheapestPlan, _: usize) -> usize {
        1
    }

    fn zero_precedes_one(_: &CheapestPlan, element: usize, out: &mut Vec<usize>) {
        if element == 0 {
            out.push(1);
        }
    }

    #[test]
    fn cheapest_precedence_hooks_order_elements_by_downstream_criticality() {
        let director = CheapestDirector::new(CheapestPlan {
            routes: vec![Vec::new()],
            score: None,
        });
        let mut solver_scope = SolverScope::new(director);
        solver_scope.start_solving();
        let mut phase = ListCheapestInsertionPhase::new(
            element_count,
            get_assigned,
            entity_count,
            list_len,
            list_insert,
            list_remove,
            swapped_index_to_element,
            0,
        )
        .with_precedence_hooks(Some(unit_duration), Some(zero_precedes_one));

        phase.solve(&mut solver_scope);

        assert_eq!(
            solver_scope.working_solution().routes,
            vec![vec![1, 0]],
            "element 0 has downstream work and should dispatch before element 1"
        );
    }
}
