//! Incremental owner-admission state for the Clarke-Wright merge loop.
//!
//! A merge is admitted when the buffered route set it produces can still be
//! matched to distinct empty owners. The working solution does not change
//! during the merge loop, so a route's feasible owners change only when the
//! route itself changes: this state keeps, beside the routes, the metric-class
//! counts, whether each route has any feasible owner, and (once few enough
//! routes remain for a matching to decide) each route's owner set. It updates
//! them for the merged route only, instead of re-deriving every route's owners
//! against every owner on each attempt. Every admit/reject decision equals the
//! full recomputation it replaces.

use std::collections::BTreeMap;

use solverforge_core::domain::PlanningSolution;

use super::owner_assignment::{
    feasible_owners_for_scored_elements, has_feasible_owner, match_route_owners, OwnerSlot,
    SavingsFeasibility,
};
use super::route_state::{route_elements_into, route_values_into, ConstructedRoute};
use super::{ClarkeWrightAccess, RuntimeListSourceIndex};

/// How a live route counts in the metric-class fast path.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClassStanding {
    /// Scored and feasible for every owner of its metric class.
    Class(usize),
    /// Unscored and feasible for every owner.
    Free,
    /// Neither: the fast path cannot admit while this route is live.
    Blocking,
}

impl ClassStanding {
    fn of(route: &ConstructedRoute) -> Self {
        match route.scored_metric_class {
            Some(metric_class) if route.feasible_for_all_metric_class_owners => {
                Self::Class(metric_class)
            }
            Some(_) => Self::Blocking,
            None if route.feasible_for_all_owners => Self::Free,
            None => Self::Blocking,
        }
    }
}

/// One merge under evaluation: route `merged` absorbs route `removed` as the
/// candidate route, scored in `metric_class`, whose feasible owners are
/// `feasible_owners`.
pub(crate) struct MergeProposal<'a> {
    pub(crate) merged: usize,
    pub(crate) removed: usize,
    pub(crate) metric_class: usize,
    pub(crate) feasible_for_all_metric_class_owners: bool,
    pub(crate) feasible_owners: &'a [usize],
}

impl MergeProposal<'_> {
    fn standing(&self) -> ClassStanding {
        if self.feasible_for_all_metric_class_owners {
            ClassStanding::Class(self.metric_class)
        } else {
            ClassStanding::Blocking
        }
    }
}

/// The frozen inputs every owner derivation reads.
pub(crate) struct OwnerQuery<'q, S, A>
where
    A: ClarkeWrightAccess<S>,
{
    pub(crate) access: &'q A,
    pub(crate) solution: &'q S,
    pub(crate) source_index: &'q RuntimeListSourceIndex<A::Element>,
    pub(crate) owner_slots: &'q [OwnerSlot],
    pub(crate) entity_count: usize,
}

pub(crate) struct OwnerAdmission {
    owner_slot_count: usize,
    owner_count_by_class: BTreeMap<usize, usize>,
    live_routes: usize,
    blocking_routes: usize,
    routes_by_class: BTreeMap<usize, usize>,
    classes_over_capacity: usize,
    /// Whether each route has no feasible owner, once known.
    ownerless: Vec<Option<bool>>,
    all_ownership_known: bool,
    /// Live routes known to have no feasible owner.
    ownerless_routes: usize,
    /// Owner sets, kept once at most one route per owner slot (plus the route
    /// a merge removes) is live, which is when a matching decides admission.
    owner_sets: Vec<Option<Vec<usize>>>,
    values_scratch: Vec<usize>,
}

impl OwnerAdmission {
    pub(crate) fn new(routes: &[ConstructedRoute], owner_slots: &[OwnerSlot]) -> Self {
        let mut owner_count_by_class = BTreeMap::new();
        for slot in owner_slots {
            *owner_count_by_class.entry(slot.metric_class).or_insert(0) += 1;
        }
        let mut admission = Self {
            owner_slot_count: owner_slots.len(),
            owner_count_by_class,
            live_routes: 0,
            blocking_routes: 0,
            routes_by_class: BTreeMap::new(),
            classes_over_capacity: 0,
            ownerless: vec![None; routes.len()],
            all_ownership_known: false,
            ownerless_routes: 0,
            owner_sets: vec![None; routes.len()],
            values_scratch: Vec::new(),
        };
        for route in routes.iter().filter(|route| !route.visits.is_empty()) {
            admission.live_routes += 1;
            admission.add_standing(ClassStanding::of(route));
        }
        admission
    }

    /// Number of owner slots in `metric_class`.
    pub(crate) fn class_owner_count(&self, metric_class: usize) -> usize {
        self.owner_count_by_class
            .get(&metric_class)
            .copied()
            .unwrap_or(0)
    }

    /// Whether the route set after `proposal` still matches owners: the
    /// metric-class count check, else every route keeps a feasible owner and,
    /// when no more routes than owners remain, a matching places them all.
    pub(crate) fn admits<S, A>(
        &mut self,
        query: &OwnerQuery<'_, S, A>,
        routes: &[ConstructedRoute],
        feasibility: &mut SavingsFeasibility,
        proposal: &MergeProposal<'_>,
    ) -> bool
    where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S>,
    {
        if self.admits_by_metric_class(routes, proposal) {
            return true;
        }
        self.learn_all_ownership(query, routes, feasibility);
        let ownerless_after = self.ownerless_routes
            - usize::from(self.is_ownerless(proposal.merged))
            - usize::from(self.is_ownerless(proposal.removed))
            + usize::from(proposal.feasible_owners.is_empty());
        if ownerless_after > 0 {
            return false;
        }
        if self.live_routes - 1 > self.owner_slot_count {
            return true;
        }
        self.derive_owner_sets(query, routes, feasibility);
        let feasible_sets = self
            .owner_sets
            .iter()
            .enumerate()
            .filter_map(|(route_idx, owners)| {
                if route_idx == proposal.merged {
                    Some(proposal.feasible_owners)
                } else if route_idx == proposal.removed {
                    None
                } else {
                    owners.as_deref()
                }
            })
            .collect::<Vec<_>>();
        match_route_owners(&feasible_sets)
            .iter()
            .all(Option::is_some)
    }

    /// Records an admitted merge before the kernel rewrites `routes`: route
    /// `proposal.merged` becomes the candidate route and `proposal.removed`
    /// becomes empty.
    pub(crate) fn commit(&mut self, routes: &[ConstructedRoute], proposal: &MergeProposal<'_>) {
        self.remove_standing(ClassStanding::of(&routes[proposal.merged]));
        self.remove_standing(ClassStanding::of(&routes[proposal.removed]));
        self.add_standing(proposal.standing());
        self.live_routes -= 1;
        self.ownerless_routes -= usize::from(self.is_ownerless(proposal.merged));
        self.ownerless_routes -= usize::from(self.is_ownerless(proposal.removed));
        let ownerless = proposal.feasible_owners.is_empty();
        self.ownerless_routes += usize::from(ownerless);
        self.ownerless[proposal.merged] = Some(ownerless);
        self.ownerless[proposal.removed] = None;
        self.owner_sets[proposal.removed] = None;
        self.owner_sets[proposal.merged] = self
            .keeps_owner_sets()
            .then(|| proposal.feasible_owners.to_vec());
    }

    /// Feasible owners of every live route, in route order.
    pub(crate) fn into_owner_sets<S, A>(
        mut self,
        query: &OwnerQuery<'_, S, A>,
        routes: &[ConstructedRoute],
        feasibility: &mut SavingsFeasibility,
    ) -> Vec<Vec<usize>>
    where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S>,
    {
        let mut elements = Vec::new();
        let mut sets = Vec::new();
        for (route_idx, route) in routes.iter().enumerate() {
            if route.visits.is_empty() {
                continue;
            }
            let owners = match self.owner_sets[route_idx].take() {
                Some(owners) => owners,
                None => self.route_owners(query, route, feasibility, &mut elements),
            };
            sets.push(owners);
        }
        sets
    }

    fn keeps_owner_sets(&self) -> bool {
        self.live_routes <= self.owner_slot_count + 1
    }

    fn admits_by_metric_class(
        &self,
        routes: &[ConstructedRoute],
        proposal: &MergeProposal<'_>,
    ) -> bool {
        let merged_before = ClassStanding::of(&routes[proposal.merged]);
        let removed_before = ClassStanding::of(&routes[proposal.removed]);
        let candidate = proposal.standing();
        let blocking_after = self.blocking_routes
            - usize::from(merged_before == ClassStanding::Blocking)
            - usize::from(removed_before == ClassStanding::Blocking)
            + usize::from(candidate == ClassStanding::Blocking);
        if blocking_after > 0 || self.live_routes - 1 > self.owner_slot_count {
            return false;
        }

        let mut affected: [Option<(usize, isize)>; 3] = [None; 3];
        for (standing, delta) in [(merged_before, -1), (removed_before, -1), (candidate, 1)] {
            let ClassStanding::Class(metric_class) = standing else {
                continue;
            };
            if let Some(entry) = affected
                .iter_mut()
                .flatten()
                .find(|(class, _)| *class == metric_class)
            {
                entry.1 += delta;
            } else if let Some(slot) = affected.iter_mut().find(|slot| slot.is_none()) {
                *slot = Some((metric_class, delta));
            }
        }

        let mut over_after = self.classes_over_capacity;
        for (metric_class, delta) in affected.into_iter().flatten() {
            let before = self
                .routes_by_class
                .get(&metric_class)
                .copied()
                .unwrap_or(0);
            let after = before
                .checked_add_signed(delta)
                .expect("a class never loses more routes than it counts");
            let capacity = self.class_owner_count(metric_class);
            over_after -= usize::from(before > capacity);
            over_after += usize::from(after > capacity);
        }
        over_after == 0
    }

    fn add_standing(&mut self, standing: ClassStanding) {
        match standing {
            ClassStanding::Class(metric_class) => {
                let capacity = self.class_owner_count(metric_class);
                let count = self.routes_by_class.entry(metric_class).or_insert(0);
                *count += 1;
                self.classes_over_capacity += usize::from(*count == capacity + 1);
            }
            ClassStanding::Free => {}
            ClassStanding::Blocking => self.blocking_routes += 1,
        }
    }

    fn remove_standing(&mut self, standing: ClassStanding) {
        match standing {
            ClassStanding::Class(metric_class) => {
                let capacity = self.class_owner_count(metric_class);
                let count = self
                    .routes_by_class
                    .get_mut(&metric_class)
                    .expect("a live classed route is counted");
                self.classes_over_capacity -= usize::from(*count == capacity + 1);
                *count -= 1;
            }
            ClassStanding::Free => {}
            ClassStanding::Blocking => self.blocking_routes -= 1,
        }
    }

    fn is_ownerless(&self, route_idx: usize) -> bool {
        self.ownerless[route_idx] == Some(true)
    }

    /// Learns, once, whether each live route has any feasible owner.
    fn learn_all_ownership<S, A>(
        &mut self,
        query: &OwnerQuery<'_, S, A>,
        routes: &[ConstructedRoute],
        feasibility: &mut SavingsFeasibility,
    ) where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S>,
    {
        if self.all_ownership_known {
            return;
        }
        let mut elements = Vec::new();
        for (route_idx, route) in routes.iter().enumerate() {
            if route.visits.is_empty() || self.ownerless[route_idx].is_some() {
                continue;
            }
            route_values_into(
                query.access,
                query.source_index,
                &route.visits,
                &mut self.values_scratch,
            );
            route_elements_into::<S, A>(query.source_index, &route.visits, &mut elements);
            let ownerless = !has_feasible_owner(
                query.access,
                query.solution,
                query.owner_slots,
                feasibility,
                &self.values_scratch,
                &elements,
                route.scored_metric_class,
                query.entity_count,
            );
            self.ownerless_routes += usize::from(ownerless);
            self.ownerless[route_idx] = Some(ownerless);
        }
        self.all_ownership_known = true;
    }

    /// Derives the owner set of every live route that has none yet.
    fn derive_owner_sets<S, A>(
        &mut self,
        query: &OwnerQuery<'_, S, A>,
        routes: &[ConstructedRoute],
        feasibility: &mut SavingsFeasibility,
    ) where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S>,
    {
        let mut elements = Vec::new();
        for (route_idx, route) in routes.iter().enumerate() {
            if route.visits.is_empty() || self.owner_sets[route_idx].is_some() {
                continue;
            }
            let owners = self.route_owners(query, route, feasibility, &mut elements);
            self.owner_sets[route_idx] = Some(owners);
        }
    }

    fn route_owners<'source, S, A>(
        &mut self,
        query: &OwnerQuery<'source, S, A>,
        route: &ConstructedRoute,
        feasibility: &mut SavingsFeasibility,
        elements: &mut Vec<&'source A::Element>,
    ) -> Vec<usize>
    where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S>,
    {
        route_values_into(
            query.access,
            query.source_index,
            &route.visits,
            &mut self.values_scratch,
        );
        route_elements_into::<S, A>(query.source_index, &route.visits, elements);
        feasible_owners_for_scored_elements(
            query.access,
            query.solution,
            query.owner_slots,
            feasibility,
            &self.values_scratch,
            elements,
            route.scored_metric_class,
            query.entity_count,
        )
    }
}
