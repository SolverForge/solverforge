use std::fmt;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{list_access::ListAccess, RuntimeListSlot};
use crate::heuristic::selector::list_kernel::{
    critical_analysis_from_graph, CriticalAnalysis, RuinSourcePool, SelectedListOwners,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::heuristic::selector::precedence_route::{
    build_precedence_route_graph, PrecedenceRouteGraph,
};
use crate::list_placement::OwnerRestriction;

/// Immutable nearby probe over the one compiled list carrier.
#[derive(Clone)]
pub(super) struct RuntimeNearbyProbe<S, V, DM, IDM> {
    slot: RuntimeListSlot<S, V, DM, IDM>,
}

/// Owned intra-route metric probe for nearby K-opt. Keeping the slot owned
/// avoids a self-referential cursor while preserving one runtime access path.
#[derive(Clone)]
pub(super) struct RuntimeKOptProbe<S, V, DM, IDM> {
    slot: RuntimeListSlot<S, V, DM, IDM>,
}

impl<S, V, DM, IDM> RuntimeKOptProbe<S, V, DM, IDM> {
    pub(super) fn new(slot: RuntimeListSlot<S, V, DM, IDM>) -> Self {
        Self { slot }
    }
}

impl<S, V, DM, IDM> crate::heuristic::selector::list_kernel::KOptDistanceProbe<S>
    for RuntimeKOptProbe<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn distance(&self, solution: &S, entity: usize, from: usize, to: usize) -> f64 {
        ListAccess::intra_position_distance(&self.slot, solution, entity, from, to)
            .expect("compiled nearby K-opt requires validated intra-position distance")
    }
}

impl<S, V, DM, IDM> RuntimeNearbyProbe<S, V, DM, IDM> {
    pub(super) fn new(slot: RuntimeListSlot<S, V, DM, IDM>) -> Self {
        Self { slot }
    }
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeNearbyProbe<S, V, DM, IDM>
where
    RuntimeListSlot<S, V, DM, IDM>: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeNearbyProbe")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<S, V, DM, IDM> RuntimeNearbyProbe<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn owner_at(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        if !self.slot.ownership_policy().is_explicit() {
            return None;
        }
        let element = ListAccess::list_get(&self.slot, solution, entity, position)?;
        Some(owner_restriction(
            &self.slot,
            solution,
            entity_count,
            &element,
        ))
    }
}

impl<S, V, DM, IDM> crate::heuristic::selector::list_kernel::NearbyChangeProbe<S>
    for RuntimeNearbyProbe<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        ListAccess::cross_position_distance(
            &self.slot,
            solution,
            source_entity,
            source_position,
            destination_entity,
            destination_position,
        )
        .expect("compiled nearby list change requires validated cross-position distance")
    }

    fn has_owner_binding(&self) -> bool {
        self.slot.ownership_policy().is_explicit()
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        self.owner_at(solution, entity_count, entity, position)
    }
}

impl<S, V, DM, IDM> crate::heuristic::selector::list_kernel::NearbySwapProbe<S>
    for RuntimeNearbyProbe<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn distance(
        &self,
        solution: &S,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        <Self as crate::heuristic::selector::list_kernel::NearbyChangeProbe<S>>::distance(
            self,
            solution,
            source_entity,
            source_position,
            destination_entity,
            destination_position,
        )
    }

    fn has_owner_binding(&self) -> bool {
        self.slot.ownership_policy().is_explicit()
    }

    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        entity: usize,
        position: usize,
    ) -> Option<OwnerRestriction> {
        self.owner_at(solution, entity_count, entity, position)
    }
}

pub(super) fn runtime_selected_owners<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    entities: &[usize],
    route_lens: &[usize],
) -> SelectedListOwners
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    if !slot.ownership_policy().is_explicit() {
        return SelectedListOwners::Absent;
    }
    let entity_count = ListAccess::entity_count(slot, solution);
    let mut fixed_to_current = true;
    let owners = entities
        .iter()
        .zip(route_lens)
        .map(|(&entity, &route_len)| {
            (0..route_len)
                .map(|position| {
                    let restriction = ListAccess::list_get(slot, solution, entity, position)
                        .map(|element| owner_restriction(slot, solution, entity_count, &element))
                        .unwrap_or(OwnerRestriction::Invalid);
                    fixed_to_current &=
                        matches!(restriction, OwnerRestriction::Fixed(owner) if owner == entity);
                    restriction
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if fixed_to_current {
        SelectedListOwners::FixedToCurrent
    } else {
        SelectedListOwners::Mixed(owners)
    }
}

pub(super) fn runtime_ruin_source_pool<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    max_source_list_len: Option<usize>,
) -> RuinSourcePool
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let entity_count = ListAccess::entity_count(slot, solution);
    let non_empty = (0..entity_count)
        .filter_map(|entity| {
            let length = ListAccess::list_len(slot, solution, entity);
            (length > 0 && max_source_list_len.is_none_or(|maximum| length <= maximum))
                .then_some((entity, length))
        })
        .collect::<Vec<_>>();
    if !slot.ownership_policy().is_explicit() {
        return RuinSourcePool::Unrestricted(non_empty);
    }
    RuinSourcePool::OwnerRestricted(
        non_empty
            .into_iter()
            .filter_map(|(entity, length)| {
                let positions = (0..length)
                    .filter(|&position| {
                        ListAccess::list_get(slot, solution, entity, position).is_some_and(
                            |element| {
                                owner_restriction(slot, solution, entity_count, &element)
                                    .allows_any(entity_count)
                            },
                        )
                    })
                    .collect::<SmallVec<[usize; 8]>>();
                (!positions.is_empty()).then_some((entity, positions))
            })
            .collect(),
    )
}

pub(super) fn runtime_precedence_analysis<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    route_graph: Option<PrecedenceRouteGraph>,
) -> Option<CriticalAnalysis>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    if !slot.precedence_policy().is_explicit() {
        return None;
    }
    let route_graph = route_graph?;
    let elements = (0..ListAccess::element_count(slot, solution))
        .map(|index| ListAccess::index_to_element(slot, solution, index))
        .collect::<Option<Vec<_>>>()?;
    let durations = elements
        .iter()
        .map(|element| {
            i64::try_from(
                ListAccess::precedence_duration(slot, solution, element.clone())
                    .expect("compiled precedence leaf requires validated duration metadata"),
            )
            .unwrap_or(i64::MAX)
        })
        .collect::<Vec<_>>();
    let entities = (0..ListAccess::entity_count(slot, solution)).collect::<Vec<_>>();
    Some(critical_analysis_from_graph(
        &durations,
        &entities,
        route_graph,
    ))
}

pub(super) fn runtime_precedence_graph<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
) -> Option<PrecedenceRouteGraph>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    if !slot.precedence_policy().has_successors() {
        return None;
    }
    let elements = (0..ListAccess::element_count(slot, solution))
        .map(|index| ListAccess::index_to_element(slot, solution, index))
        .collect::<Option<Vec<_>>>()?;
    Some(build_precedence_route_graph(
        solution,
        &elements,
        |solution, element, successors| {
            ListAccess::extend_precedence_successors(slot, solution, element, successors)
                .expect("compiled precedence leaf requires validated successor metadata")
        },
        |solution| ListAccess::entity_count(slot, solution),
        |solution, entity| ListAccess::list_len(slot, solution, entity),
        |solution, entity, position| ListAccess::list_get(slot, solution, entity, position),
    ))
}

fn owner_restriction<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    entity_count: usize,
    element: &<RuntimeListSlot<S, V, DM, IDM> as ListAccess<S>>::Element,
) -> OwnerRestriction
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    match ListAccess::element_owner(slot, solution, element)
        .expect("compiled owner-aware list leaf requires validated owner metadata")
    {
        Some(owner) if owner < entity_count => OwnerRestriction::Fixed(owner),
        Some(_) => OwnerRestriction::Invalid,
        None => OwnerRestriction::Unrestricted,
    }
}

trait OwnerRestrictionExt {
    fn allows_any(self, entity_count: usize) -> bool;
}

impl OwnerRestrictionExt for OwnerRestriction {
    fn allows_any(self, entity_count: usize) -> bool {
        (0..entity_count).any(|entity| self.allows(entity))
    }
}
