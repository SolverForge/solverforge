use solverforge_core::domain::PlanningSolution;

use super::{ClarkeWrightAccess, RuntimeListSourceIndex};

#[derive(Clone)]
pub(crate) struct ConstructedRoute {
    pub(crate) visits: Vec<usize>,
    pub(crate) scored_metric_class: Option<usize>,
    pub(crate) feasible_for_all_owners: bool,
    pub(crate) feasible_for_all_metric_class_owners: bool,
}

impl ConstructedRoute {
    pub(crate) fn singleton(element_idx: usize, feasible_for_all_owners: bool) -> Self {
        Self {
            visits: vec![element_idx],
            scored_metric_class: None,
            feasible_for_all_owners,
            feasible_for_all_metric_class_owners: false,
        }
    }

    pub(crate) fn can_merge_for_metric_class(&self, metric_class: usize) -> bool {
        self.scored_metric_class
            .is_none_or(|scored_metric_class| scored_metric_class == metric_class)
    }
}

pub(crate) fn route_values<S, A>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    route: &[usize],
) -> Vec<usize>
where
    A: ClarkeWrightAccess<S>,
{
    route
        .iter()
        .map(|&idx| access.route_value(source_index.element(idx)))
        .collect()
}

pub(crate) fn route_values_into<S, A>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    route: &[usize],
    values: &mut Vec<usize>,
) where
    A: ClarkeWrightAccess<S>,
{
    values.clear();
    values.extend(
        route
            .iter()
            .map(|&idx| access.route_value(source_index.element(idx))),
    );
}

pub(crate) fn route_elements_into<'source, S, A>(
    source_index: &'source RuntimeListSourceIndex<A::Element>,
    route: &[usize],
    elements: &mut Vec<&'source A::Element>,
) where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
{
    elements.clear();
    elements.extend(route.iter().map(|&idx| source_index.element(idx)));
}
