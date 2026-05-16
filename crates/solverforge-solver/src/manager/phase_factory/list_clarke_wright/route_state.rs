use solverforge_core::domain::PlanningSolution;

use super::owner_assignment::{feasible_owners_for_scored_route, OwnerSlot};

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

pub(crate) fn route_values<S, E>(
    solution: &S,
    index_to_element: fn(&S, usize) -> E,
    route: &[usize],
) -> Vec<usize>
where
    E: Copy + Into<usize>,
{
    route
        .iter()
        .map(|&idx| index_to_element(solution, idx).into())
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn routes_match_owners_after_merge<S, E>(
    solution: &S,
    routes: &[ConstructedRoute],
    merged_route_idx: usize,
    removed_route_idx: usize,
    candidate_route: &[usize],
    candidate_metric_class: usize,
    candidate_feasible_for_all_metric_class_owners: bool,
    owner_slots: &[OwnerSlot],
    index_to_element: fn(&S, usize) -> E,
    feasible: fn(&S, usize, &[usize]) -> bool,
) -> bool
where
    S: PlanningSolution,
    E: Copy + Into<usize>,
{
    if routes_match_owners_by_metric_class(
        routes,
        merged_route_idx,
        removed_route_idx,
        candidate_metric_class,
        candidate_feasible_for_all_metric_class_owners,
        owner_slots,
    ) {
        return true;
    }

    let mut feasible_sets = Vec::new();
    for (route_idx, route) in routes.iter().enumerate() {
        let (route, scored_metric_class) = if route_idx == merged_route_idx {
            (candidate_route, Some(candidate_metric_class))
        } else if route_idx == removed_route_idx || route.visits.is_empty() {
            continue;
        } else {
            (route.visits.as_slice(), route.scored_metric_class)
        };
        let values = route_values(solution, index_to_element, route);
        let feasible_owners = feasible_owners_for_scored_route(
            solution,
            owner_slots,
            &values,
            scored_metric_class,
            feasible,
        );
        if feasible_owners.is_empty() {
            return false;
        }
        feasible_sets.push(feasible_owners);
    }

    if feasible_sets.len() > owner_slots.len() {
        return true;
    }

    super::owner_assignment::match_route_owners(&feasible_sets)
        .iter()
        .all(Option::is_some)
}

fn routes_match_owners_by_metric_class(
    routes: &[ConstructedRoute],
    merged_route_idx: usize,
    removed_route_idx: usize,
    candidate_metric_class: usize,
    candidate_feasible_for_all_metric_class_owners: bool,
    owner_slots: &[OwnerSlot],
) -> bool {
    let mut route_count_by_metric_class = std::collections::BTreeMap::new();
    let mut owner_count_by_metric_class = std::collections::BTreeMap::new();
    let mut non_empty_route_count = 0usize;

    for slot in owner_slots {
        *owner_count_by_metric_class
            .entry(slot.metric_class)
            .or_insert(0usize) += 1;
    }

    for (route_idx, route) in routes.iter().enumerate() {
        if route_idx == removed_route_idx || route.visits.is_empty() {
            continue;
        }

        non_empty_route_count += 1;
        let (scored_metric_class, feasible_for_all_metric_class_owners) =
            if route_idx == merged_route_idx {
                (
                    Some(candidate_metric_class),
                    candidate_feasible_for_all_metric_class_owners,
                )
            } else {
                (
                    route.scored_metric_class,
                    route.feasible_for_all_metric_class_owners,
                )
            };

        match scored_metric_class {
            Some(metric_class) if feasible_for_all_metric_class_owners => {
                *route_count_by_metric_class
                    .entry(metric_class)
                    .or_insert(0usize) += 1;
            }
            Some(_) => {
                return false;
            }
            None if route.feasible_for_all_owners => {}
            None => return false,
        }
    }

    if non_empty_route_count > owner_slots.len() {
        return false;
    }

    route_count_by_metric_class
        .into_iter()
        .all(|(metric_class, route_count)| {
            owner_count_by_metric_class
                .get(&metric_class)
                .is_some_and(|owner_count| route_count <= *owner_count)
        })
}
