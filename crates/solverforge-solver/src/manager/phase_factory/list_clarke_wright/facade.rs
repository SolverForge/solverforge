//! Function-pointer adapter that drives the live kernel in tests.

use super::*;
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListSourceAccess,
};
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

/// Public facade for the canonical Clarke-Wright construction kernel.
///
/// Its constructor requires an explicit stable source key. Declaration and
/// assignment identity are therefore validated before construction without
/// payload equality/hash recovery. `Into<usize>` has a separate, explicit
/// role here: it supplies CVRP route values for savings calculations, never
/// construction identity.
pub(crate) struct ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    element_count: fn(&S) -> usize,
    get_assigned: fn(&S) -> Vec<E>,
    entity_count: fn(&S) -> usize,
    route_len: fn(&S, usize) -> usize,
    assign_route: fn(&mut S, usize, Vec<usize>),
    index_to_element: fn(&S, usize) -> E,
    element_source_key: fn(&S, &E) -> usize,
    depot_fn: fn(&S, usize) -> usize,
    metric_class_fn: fn(&S, usize) -> usize,
    distance_fn: fn(&S, usize, usize, usize) -> i64,
    feasible_fn: fn(&S, usize, &[usize]) -> bool,
    element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    descriptor_index: usize,
}

impl<S, E> std::fmt::Debug for ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListClarkeWrightPhase").finish()
    }
}

impl<S, E> ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        element_count: fn(&S) -> usize,
        get_assigned: fn(&S) -> Vec<E>,
        entity_count: fn(&S) -> usize,
        route_len: fn(&S, usize) -> usize,
        assign_route: fn(&mut S, usize, Vec<usize>),
        index_to_element: fn(&S, usize) -> E,
        element_source_key: fn(&S, &E) -> usize,
        depot_fn: fn(&S, usize) -> usize,
        distance_fn: fn(&S, usize, usize, usize) -> i64,
        feasible_fn: fn(&S, usize, &[usize]) -> bool,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            get_assigned,
            entity_count,
            route_len,
            assign_route,
            index_to_element,
            element_source_key,
            depot_fn,
            metric_class_fn: unique_metric_class::<S>,
            distance_fn,
            feasible_fn,
            element_owner_fn: None,
            descriptor_index,
        }
    }

    pub(crate) fn with_metric_class_fn(mut self, metric_class_fn: fn(&S, usize) -> usize) -> Self {
        self.metric_class_fn = metric_class_fn;
        self
    }

    pub(crate) fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

impl<S, E> ListSourceAccess<S> for ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
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

impl<S, E> ClarkeWrightAccess<S> for ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
{
    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_count(&self, solution: &S) -> usize {
        (self.entity_count)(solution)
    }

    fn route_len(&self, solution: &S, entity_index: usize) -> usize {
        (self.route_len)(solution, entity_index)
    }

    fn route_value(&self, element: &Self::Element) -> usize {
        (*element).into()
    }

    fn element_owner(&self, solution: &S, element: &Self::Element) -> Option<usize> {
        self.element_owner_fn
            .and_then(|owner| owner(solution, element))
    }

    fn savings_depot(&self, solution: &S, entity_index: usize) -> usize {
        (self.depot_fn)(solution, entity_index)
    }

    fn savings_metric_class(&self, solution: &S, entity_index: usize) -> usize {
        (self.metric_class_fn)(solution, entity_index)
    }

    fn savings_distance(&self, solution: &S, entity_index: usize, from: usize, to: usize) -> i64 {
        (self.distance_fn)(solution, entity_index, from, to)
    }

    fn savings_feasible(&self, solution: &S, entity_index: usize, route: &[usize]) -> bool {
        (self.feasible_fn)(solution, entity_index, route)
    }

    fn replace_route(&self, solution: &mut S, entity_index: usize, route: Vec<usize>) {
        (self.assign_route)(solution, entity_index, route);
    }
}

impl<S, E, D, BestCb> Phase<S, D, BestCb> for ListClarkeWrightPhase<S, E>
where
    S: PlanningSolution,
    E: Copy + Into<usize> + Send + Sync + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        let binding = bind_runtime_list_source(self, solver_scope.working_solution())
            .unwrap_or_else(|error| {
                panic!("ListClarkeWright source binding failed before phase execution: {error:?}")
            });
        let source_index = binding.into_source_index();
        let unassigned = unassigned_from_current_assignment(
            self,
            &source_index,
            solver_scope.working_solution(),
        )
        .unwrap_or_else(|error| {
            panic!("ListClarkeWright assignment refresh failed before phase execution: {error:?}")
        });
        run_clarke_wright(
            self,
            &source_index,
            &unassigned,
            StepControlPolicy::ObserveConfigLimits,
            solver_scope,
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "ListClarkeWright"
    }
}
