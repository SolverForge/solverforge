//! The public Clarke-Wright facade and its one canonical access protocol.
//!
//! The savings/merge/matching/completion algorithm lives in `kernel.rs`.
//! Both this established public phase and compiled `RuntimeListSlot` instances
//! adapt to that one kernel; neither owns a second construction algorithm.

use solverforge_core::domain::PlanningSolution;

use super::distance_arithmetic::sum_two_minus_one;
use crate::builder::context::list_access::{ListAccess, RouteSequenceAccess, SavingsAccess};
use crate::builder::context::{
    ListSourceAccess, RuntimeListElement, RuntimeListSlot, RuntimeListSourceIndex, SourceElement,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

mod commit;
mod completion;
#[cfg(test)]
mod facade;
mod kernel;
mod owner_assignment;
mod route_state;
mod savings;

#[cfg(test)]
pub(crate) use facade::ListClarkeWrightPhase;
pub(crate) use kernel::run_clarke_wright;

/// The declaration-resolved operations used by canonical Clarke-Wright.
///
/// The type is deliberately small: it exposes exactly the source, savings,
/// owner, and route-replacement semantics that the algorithm consumes. It is
/// implemented by the public function-pointer facade and by `RuntimeListSlot`.
/// Thus static and dynamic models run the same savings ordering, merge logic,
/// completion logic, and trace sources.
pub(crate) trait ClarkeWrightAccess<S>: ListSourceAccess<S> {
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
    route: &[&A::Element],
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
    route_values_by_source_index: &[usize],
) -> i64
where
    A: ClarkeWrightAccess<S>,
{
    let depot = access.savings_depot(solution, owner_idx);
    let value = route_values_by_source_index[element_idx];
    let previous = if insert_idx == 0 {
        depot
    } else {
        route_values_by_source_index[route_indices[insert_idx - 1]]
    };
    let next = route_indices
        .get(insert_idx)
        .map(|&idx| route_values_by_source_index[idx])
        .unwrap_or(depot);
    sum_two_minus_one(
        access.savings_distance(solution, owner_idx, previous, value),
        access.savings_distance(solution, owner_idx, value, next),
        access.savings_distance(solution, owner_idx, previous, next),
    )
}

#[cfg(test)]
fn unique_metric_class<S>(_: &S, owner_idx: usize) -> usize {
    owner_idx
}

#[cfg(test)]
mod tests;
