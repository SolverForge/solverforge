use std::collections::{BTreeMap, BTreeSet};

use solverforge_core::domain::PlanningSolution;

pub(crate) fn feasible_owners<S>(
    solution: &S,
    available_entity_slots: &[usize],
    route: &[usize],
    feasible: fn(&S, usize, &[usize]) -> bool,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    available_entity_slots
        .iter()
        .copied()
        .filter(|&entity_idx| feasible(solution, entity_idx, route))
        .collect()
}

pub(crate) fn match_route_owners(feasible_sets: &[Vec<usize>]) -> Vec<Option<usize>> {
    let mut route_order: Vec<usize> = (0..feasible_sets.len()).collect();
    route_order.sort_by_key(|&route_idx| (feasible_sets[route_idx].len(), route_idx));

    let mut owner_to_route: BTreeMap<usize, usize> = BTreeMap::new();
    for route_idx in route_order {
        let mut seen = BTreeSet::new();
        let _ = assign_route(route_idx, feasible_sets, &mut owner_to_route, &mut seen);
    }

    let mut route_to_owner = vec![None; feasible_sets.len()];
    for (owner_idx, route_idx) in owner_to_route {
        route_to_owner[route_idx] = Some(owner_idx);
    }
    route_to_owner
}

fn assign_route(
    route_idx: usize,
    feasible_sets: &[Vec<usize>],
    owner_to_route: &mut BTreeMap<usize, usize>,
    seen: &mut BTreeSet<usize>,
) -> bool {
    for &owner_idx in &feasible_sets[route_idx] {
        if !seen.insert(owner_idx) {
            continue;
        }

        let displaced = owner_to_route.get(&owner_idx).copied();
        if displaced.is_none_or(|existing_route| {
            assign_route(existing_route, feasible_sets, owner_to_route, seen)
        }) {
            owner_to_route.insert(owner_idx, route_idx);
            return true;
        }
    }

    false
}
