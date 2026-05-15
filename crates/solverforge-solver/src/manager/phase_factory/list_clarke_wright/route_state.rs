use solverforge_core::domain::PlanningSolution;

use super::owner_assignment::feasible_owners_for_scored_route;

pub(crate) struct ConstructedRoute {
    pub(crate) visits: Vec<usize>,
    pub(crate) scored_owner: Option<usize>,
}

impl ConstructedRoute {
    pub(crate) fn singleton(element_idx: usize) -> Self {
        Self {
            visits: vec![element_idx],
            scored_owner: None,
        }
    }

    pub(crate) fn can_merge_for_owner(&self, owner_idx: usize) -> bool {
        self.scored_owner
            .is_none_or(|scored_owner| scored_owner == owner_idx)
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
    candidate_owner_idx: usize,
    available_entity_slots: &[usize],
    index_to_element: fn(&S, usize) -> E,
    feasible: fn(&S, usize, &[usize]) -> bool,
) -> bool
where
    S: PlanningSolution,
    E: Copy + Into<usize>,
{
    let mut feasible_sets = Vec::new();
    for (route_idx, route) in routes.iter().enumerate() {
        let (route, scored_owner) = if route_idx == merged_route_idx {
            (candidate_route, Some(candidate_owner_idx))
        } else if route_idx == removed_route_idx || route.visits.is_empty() {
            continue;
        } else {
            (route.visits.as_slice(), route.scored_owner)
        };
        let values = route_values(solution, index_to_element, route);
        let feasible_owners = feasible_owners_for_scored_route(
            solution,
            available_entity_slots,
            &values,
            scored_owner,
            feasible,
        );
        if feasible_owners.is_empty() {
            return false;
        }
        feasible_sets.push(feasible_owners);
    }

    if feasible_sets.len() > available_entity_slots.len() {
        return true;
    }

    super::owner_assignment::match_route_owners(&feasible_sets)
        .iter()
        .all(Option::is_some)
}
