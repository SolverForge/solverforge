//! Function-pointer adapter that drives the live kernel in tests.

use super::*;
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListSourceAccess,
};
use crate::list_placement::OwnerRestriction;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use std::marker::PhantomData;

/// List construction phase that assigns elements round-robin to entities.
pub(crate) struct ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    assign_element: fn(&mut S, usize, E),
    index_to_element: fn(&S, usize) -> E,
    element_source_key: fn(&S, &E) -> usize,
    descriptor_index: usize,
    _marker: PhantomData<(fn() -> S, fn() -> E)>,
}

impl<S, E> ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    pub(crate) fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        assign_element: fn(&mut S, usize, E),
        index_to_element: fn(&S, usize) -> E,
        element_source_key: fn(&S, &E) -> usize,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            assign_element,
            index_to_element,
            element_source_key,
            descriptor_index,
            _marker: PhantomData,
        }
    }
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

impl<S, E> ListSourceAccess<S> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    type Element = E;

    fn element_count(&self, solution: &S) -> usize {
        (self.element_count)(solution)
    }

    fn index_to_element(&self, solution: &S, source_index: usize) -> Option<Self::Element> {
        (source_index < (self.element_count)(solution))
            .then(|| (self.index_to_element)(solution, source_index))
    }

    fn element_source_key(&self, solution: &S, element: &Self::Element) -> usize {
        (self.element_source_key)(solution, element)
    }

    fn assigned_elements(&self, solution: &S) -> Vec<Self::Element> {
        (self.get_assigned)(solution)
    }
}

impl<S, E> RoundRobinAccess<S> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
{
    type Element = E;

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64 {
        let _ = (solution, element);
        0
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction {
        let _ = (solution, entity_count, element);
        OwnerRestriction::Unrestricted
    }

    fn append_element(&self, solution: &mut S, entity_index: usize, element: Self::Element) {
        (self.assign_element)(solution, entity_index, element);
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListConstructionPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Send + Sync + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        let binding = bind_runtime_list_source(self, solver_scope.working_solution())
            .unwrap_or_else(|error| {
                panic!(
                    "ListConstructionPhase source binding failed before phase execution: {error:?}"
                )
            });
        let source_index = binding.into_source_index();
        let unassigned = unassigned_from_current_assignment(
            self,
            &source_index,
            solver_scope.working_solution(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "ListConstructionPhase assignment refresh failed before phase execution: {error:?}"
            )
        });
        let all_assigned = unassigned.is_empty();
        run_round_robin(
            self,
            source_index.source_count(),
            all_assigned,
            &unassigned,
            StepControlPolicy::ObserveConfigLimits,
            solver_scope,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListConstruction"
    }
}
