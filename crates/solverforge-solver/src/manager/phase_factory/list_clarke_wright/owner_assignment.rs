use std::collections::{BTreeMap, BTreeSet};

use solverforge_core::domain::PlanningSolution;

use super::route_state::{route_values, ConstructedRoute};
use super::{route_restriction, ClarkeWrightAccess, RuntimeListSourceIndex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OwnerSlot {
    pub(crate) owner_idx: usize,
    pub(crate) metric_class: usize,
}

pub(crate) fn owner_slots<S, A>(
    access: &A,
    solution: &S,
    available_entity_slots: &[usize],
) -> Vec<OwnerSlot>
where
    A: ClarkeWrightAccess<S>,
{
    available_entity_slots
        .iter()
        .copied()
        .map(|owner_idx| OwnerSlot {
            owner_idx,
            metric_class: access.savings_metric_class(solution, owner_idx),
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

/// Answers `savings_feasible` for owner slots, one route at a time.
///
/// When the access declares savings feasibility shared by metric class, the
/// first owner of a class answers for every owner of that class; otherwise each
/// owner is asked. Either way the answers equal asking every owner.
pub(crate) struct SavingsFeasibility {
    shared_by_metric_class: bool,
    class_position: Vec<usize>,
    answers: Vec<Option<bool>>,
}

impl SavingsFeasibility {
    pub(crate) fn new<S, A>(access: &A, owner_slots: &[OwnerSlot]) -> Self
    where
        A: ClarkeWrightAccess<S>,
    {
        let mut positions = BTreeMap::new();
        let class_position = owner_slots
            .iter()
            .map(|slot| {
                let next = positions.len();
                *positions.entry(slot.metric_class).or_insert(next)
            })
            .collect();
        Self {
            shared_by_metric_class: access.savings_feasible_shared_by_metric_class(),
            class_position,
            answers: vec![None; positions.len()],
        }
    }

    /// Starts answering for a new route.
    pub(crate) fn begin_route(&mut self) {
        if self.shared_by_metric_class {
            self.answers.fill(None);
        }
    }

    /// Whether the owner at `slot_position` admits the current route.
    pub(crate) fn feasible<S, A>(
        &mut self,
        access: &A,
        solution: &S,
        owner_slots: &[OwnerSlot],
        slot_position: usize,
        route: &[usize],
    ) -> bool
    where
        A: ClarkeWrightAccess<S>,
    {
        let owner_idx = owner_slots[slot_position].owner_idx;
        if !self.shared_by_metric_class {
            return access.savings_feasible(solution, owner_idx, route);
        }
        let answer = &mut self.answers[self.class_position[slot_position]];
        *answer.get_or_insert_with(|| access.savings_feasible(solution, owner_idx, route))
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn feasible_owners_for_scored_elements<S, A>(
    access: &A,
    solution: &S,
    owner_slots: &[OwnerSlot],
    feasibility: &mut SavingsFeasibility,
    route_values: &[usize],
    route_elements: &[&A::Element],
    scored_metric_class: Option<usize>,
    entity_count: usize,
) -> Vec<usize>
where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
{
    feasibility.begin_route();
    let restriction = route_restriction(access, solution, entity_count, route_elements);
    let mut owners = Vec::new();
    for (slot_position, slot) in owner_slots.iter().enumerate() {
        if scored_metric_class.is_some_and(|class| slot.metric_class != class) {
            continue;
        }
        if feasibility.feasible(access, solution, owner_slots, slot_position, route_values)
            && restriction.allows(slot.owner_idx)
        {
            owners.push(slot.owner_idx);
        }
    }
    owners
}

/// Whether any owner admits the route; the early-exit form of
/// [`feasible_owners_for_scored_elements`]`(..).is_empty()`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn has_feasible_owner<S, A>(
    access: &A,
    solution: &S,
    owner_slots: &[OwnerSlot],
    feasibility: &mut SavingsFeasibility,
    route_values: &[usize],
    route_elements: &[&A::Element],
    scored_metric_class: Option<usize>,
    entity_count: usize,
) -> bool
where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
{
    feasibility.begin_route();
    let restriction = route_restriction(access, solution, entity_count, route_elements);
    owner_slots
        .iter()
        .enumerate()
        .filter(|(_, slot)| scored_metric_class.is_none_or(|class| slot.metric_class == class))
        .any(|(slot_position, slot)| {
            feasibility.feasible(access, solution, owner_slots, slot_position, route_values)
                && restriction.allows(slot.owner_idx)
        })
}

pub(crate) fn match_route_owners<T>(feasible_sets: &[T]) -> Vec<Option<usize>>
where
    T: AsRef<[usize]>,
{
    let mut route_order: Vec<usize> = (0..feasible_sets.len()).collect();
    route_order.sort_by_key(|&route_idx| (feasible_sets[route_idx].as_ref().len(), route_idx));

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

/// Assigns buffered construction routes to distinct empty owner slots without
/// probing owner feasibility again.
///
/// The merge loop only admits a merge once the whole buffered route set still
/// matches owners by metric class, so every scored route carries
/// `feasible_for_all_metric_class_owners` and every unscored route carries
/// `feasible_for_all_owners` — both verified against the same owner slots. That
/// uniformity is what lets an interrupted construction publish its buffered
/// merges: the assignment needs class counting, not another feasibility scan.
///
/// Returns `None` when a buffered route cannot be placed from that evidence, so
/// the caller keeps whatever complete assignment is already committed.
pub(crate) fn assign_buffered_routes<S, A>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    owner_slots: &[OwnerSlot],
    available_entity_slots: &[usize],
    routes: &[ConstructedRoute],
) -> Option<Vec<(usize, Vec<usize>)>>
where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
{
    let mut free_owners_by_class: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for slot in owner_slots {
        free_owners_by_class
            .entry(slot.metric_class)
            .or_default()
            .push(slot.owner_idx);
    }
    let mut available_slots = available_entity_slots
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();

    let mut completed_routes = Vec::with_capacity(routes.len());
    let mut unscored = Vec::new();
    for route in routes {
        if route.visits.is_empty() {
            continue;
        }
        match route.scored_metric_class {
            Some(metric_class) if route.feasible_for_all_metric_class_owners => {
                let owner_idx = *free_owners_by_class.get(&metric_class)?.last()?;
                let owners = free_owners_by_class.get_mut(&metric_class)?;
                owners.pop();
                if !available_slots.remove(&owner_idx) {
                    return None;
                }
                completed_routes
                    .push((owner_idx, route_values(access, source_index, &route.visits)));
            }
            None if route.feasible_for_all_owners => unscored.push(route),
            _ => return None,
        }
    }

    // An unscored route accepts every owner, so it takes any slot still free
    // once every metric-class-constrained route has been placed.
    for route in unscored {
        let owner_idx = free_owners_by_class
            .values_mut()
            .find_map(|owners| owners.pop())?;
        if !available_slots.remove(&owner_idx) {
            return None;
        }
        completed_routes.push((owner_idx, route_values(access, source_index, &route.visits)));
    }

    Some(completed_routes)
}

fn assign_route<T>(
    route_idx: usize,
    feasible_sets: &[T],
    owner_to_route: &mut BTreeMap<usize, usize>,
    seen: &mut BTreeSet<usize>,
) -> bool
where
    T: AsRef<[usize]>,
{
    for &owner_idx in feasible_sets[route_idx].as_ref() {
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
