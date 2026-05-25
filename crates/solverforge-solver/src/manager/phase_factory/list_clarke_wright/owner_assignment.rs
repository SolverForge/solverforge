use std::collections::{BTreeMap, BTreeSet};

use solverforge_core::domain::PlanningSolution;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnerSlot {
    pub(crate) owner_idx: usize,
    pub(crate) metric_class: usize,
}

pub(crate) fn owner_slots<S>(
    solution: &S,
    available_entity_slots: &[usize],
    metric_class: fn(&S, usize) -> usize,
) -> Vec<OwnerSlot> {
    available_entity_slots
        .iter()
        .copied()
        .map(|owner_idx| OwnerSlot {
            owner_idx,
            metric_class: metric_class(solution, owner_idx),
        })
        .collect()
}

pub(crate) fn representative_owner_slots(owner_slots: &[OwnerSlot]) -> Vec<OwnerSlot> {
    let mut representatives = BTreeMap::new();
    for &slot in owner_slots {
        representatives
            .entry(slot.metric_class)
            .or_insert(slot.owner_idx);
    }

    representatives
        .into_iter()
        .map(|(metric_class, owner_idx)| OwnerSlot {
            owner_idx,
            metric_class,
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn feasible_owners_for_scored_elements<S, E>(
    solution: &S,
    owner_slots: &[OwnerSlot],
    route_values: &[usize],
    route_elements: Option<&[E]>,
    scored_metric_class: Option<usize>,
    feasible: fn(&S, usize, &[usize]) -> bool,
    element_owner_fn: Option<fn(&S, &E) -> Option<usize>>,
    entity_count: usize,
) -> Vec<usize>
where
    S: PlanningSolution,
{
    owner_slots
        .iter()
        .filter(|slot| scored_metric_class.is_none_or(|class| slot.metric_class == class))
        .map(|slot| slot.owner_idx)
        .filter(|&entity_idx| {
            if !feasible(solution, entity_idx, route_values) {
                return false;
            }

            let Some(owner_fn) = element_owner_fn else {
                return true;
            };
            let Some(route_elements) = route_elements else {
                return false;
            };

            crate::list_placement::route_owner_allows(
                Some(owner_fn),
                solution,
                entity_count,
                entity_idx,
                route_elements,
            )
        })
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
