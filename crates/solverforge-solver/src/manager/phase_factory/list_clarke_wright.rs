//! The public Clarke-Wright facade and its one canonical access protocol.
//!
//! The savings/merge/matching/completion algorithm lives in `kernel.rs`.
//! Both this established public phase and compiled `RuntimeListSlot` instances
//! adapt to that one kernel; neither owns a second construction algorithm.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::distance_arithmetic::sum_two_minus_one;
use crate::builder::context::list_access::{ListAccess, RouteSequenceAccess, SavingsAccess};
use crate::builder::context::{
    bind_runtime_list_source, unassigned_from_current_assignment, ListSourceAccess,
    RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex, SourceElement,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::Phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};

mod completion;
mod kernel;
mod owner_assignment;
mod route_state;
mod savings;

pub(crate) use kernel::run_clarke_wright;

/// The declaration-resolved operations used by canonical Clarke-Wright.
///
/// The type is deliberately small: it exposes exactly the source, savings,
/// owner, and route-replacement semantics that the algorithm consumes. It is
/// implemented by the public function-pointer facade and by `RuntimeListSlot`.
/// Thus static and dynamic models run the same savings ordering, merge logic,
/// completion logic, and trace sources.
pub(crate) trait ClarkeWrightAccess<S>: ListSourceAccess<S> {
    fn entity_type_name(&self) -> &'static str;
    fn variable_name(&self) -> &'static str;
    fn descriptor_index(&self) -> usize;
    fn entity_count(&self, solution: &S) -> usize;
    fn route_len(&self, solution: &S, entity_index: usize) -> usize;
    fn route_value(&self, element: &Self::Element) -> usize;
    fn element_owner(&self, solution: &S, element: &Self::Element) -> Option<usize>;
    fn savings_depot(&self, solution: &S, entity_index: usize) -> usize;
    fn savings_metric_class(&self, solution: &S, entity_index: usize) -> usize;
    fn savings_distance(&self, solution: &S, entity_index: usize, from: usize, to: usize) -> i64;
    fn savings_feasible(&self, solution: &S, entity_index: usize, route: &[usize]) -> bool;
    fn replace_route(&self, solution: &mut S, entity_index: usize, route: Vec<usize>);
}

impl<S, V, DM, IDM> ClarkeWrightAccess<S> for RuntimeListSlot<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Into<usize> + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn entity_type_name(&self) -> &'static str {
        ListAccess::entity_type_name(self)
    }

    fn variable_name(&self) -> &'static str {
        ListAccess::variable_name(self)
    }

    fn descriptor_index(&self) -> usize {
        ListAccess::descriptor_index(self)
    }

    fn entity_count(&self, solution: &S) -> usize {
        ListAccess::entity_count(self, solution)
    }

    fn route_len(&self, solution: &S, entity_index: usize) -> usize {
        ListAccess::list_len(self, solution, entity_index)
    }

    fn route_value(&self, element: &Self::Element) -> usize {
        match element {
            RuntimeListElement::Static(value) => value.clone().into(),
            RuntimeListElement::Dynamic(value) => *value,
        }
    }

    fn element_owner(&self, solution: &S, element: &Self::Element) -> Option<usize> {
        ListAccess::element_owner(self, solution, element)
            .expect("compiled Clarke-Wright ownership policy must be callable")
    }

    fn savings_depot(&self, solution: &S, entity_index: usize) -> usize {
        SavingsAccess::savings_depot(self, solution, entity_index)
            .expect("compiled Clarke-Wright savings depot must be callable")
    }

    fn savings_metric_class(&self, solution: &S, entity_index: usize) -> usize {
        SavingsAccess::savings_metric_class(self, solution, entity_index)
            .expect("compiled Clarke-Wright savings metric class must be callable")
    }

    fn savings_distance(&self, solution: &S, entity_index: usize, from: usize, to: usize) -> i64 {
        SavingsAccess::savings_distance(self, solution, entity_index, from, to)
            .expect("compiled Clarke-Wright savings distance must be callable")
    }

    fn savings_feasible(&self, solution: &S, entity_index: usize, route: &[usize]) -> bool {
        SavingsAccess::savings_feasible(self, solution, entity_index, route)
            .expect("compiled Clarke-Wright savings feasibility must be callable")
    }

    fn replace_route(&self, solution: &mut S, entity_index: usize, route: Vec<usize>) {
        RouteSequenceAccess::replace_route(self, solution, entity_index, route)
            .expect("compiled Clarke-Wright route replacement must be callable");
    }
}

pub(super) fn owner_allows<S, A>(
    access: &A,
    solution: &S,
    entity_count: usize,
    entity_index: usize,
    element: &A::Element,
) -> bool
where
    A: ClarkeWrightAccess<S>,
{
    match access.element_owner(solution, element) {
        None => true,
        Some(owner_index) => owner_index < entity_count && owner_index == entity_index,
    }
}

pub(super) fn route_owner_allows<S, A>(
    access: &A,
    solution: &S,
    entity_count: usize,
    entity_index: usize,
    route: &[A::Element],
) -> bool
where
    A: ClarkeWrightAccess<S>,
{
    route
        .iter()
        .all(|element| owner_allows(access, solution, entity_count, entity_index, element))
}

pub(super) struct CompletionAssignment {
    pub(super) owner_idx: usize,
    pub(super) route_indices: Vec<usize>,
}

pub(super) fn insertion_delta<S, A>(
    solution: &S,
    owner_idx: usize,
    route_indices: &[usize],
    insert_idx: usize,
    element_idx: usize,
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
) -> i64
where
    A: ClarkeWrightAccess<S>,
{
    let depot = access.savings_depot(solution, owner_idx);
    let value = access.route_value(source_index.element(element_idx));
    let previous = if insert_idx == 0 {
        depot
    } else {
        access.route_value(source_index.element(route_indices[insert_idx - 1]))
    };
    let next = route_indices
        .get(insert_idx)
        .map(|&idx| access.route_value(source_index.element(idx)))
        .unwrap_or(depot);
    sum_two_minus_one(
        access.savings_distance(solution, owner_idx, previous, value),
        access.savings_distance(solution, owner_idx, value, next),
        access.savings_distance(solution, owner_idx, previous, next),
    )
}

fn unique_metric_class<S>(_: &S, owner_idx: usize) -> usize {
    owner_idx
}

/// Public facade for the canonical Clarke-Wright construction kernel.
///
/// Its constructor requires an explicit stable source key. Declaration and
/// assignment identity are therefore validated before construction without
/// payload equality/hash recovery. `Into<usize>` has a separate, explicit
/// role here: it supplies CVRP route values for savings calculations, never
/// construction identity.
pub struct ListClarkeWrightPhase<S, E>
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
    pub fn new(
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

    pub fn with_metric_class_fn(mut self, metric_class_fn: fn(&S, usize) -> usize) -> Self {
        self.metric_class_fn = metric_class_fn;
        self
    }

    pub fn with_element_owner_fn(
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
    fn entity_type_name(&self) -> &'static str {
        "list_clarke_wright"
    }

    fn variable_name(&self) -> &'static str {
        "list_clarke_wright"
    }

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
                panic!(
                    "ListClarkeWright {}.{} source binding failed before phase execution: {error:?}",
                    self.entity_type_name(),
                    self.variable_name(),
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
                "ListClarkeWright {}.{} assignment refresh failed before phase execution: {error:?}",
                self.entity_type_name(),
                self.variable_name(),
            )
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

#[cfg(test)]
mod tests;
