use std::any::TypeId;
use std::sync::Arc;

use solverforge_config::{
    KOptMoveSelectorConfig, ListChangeMoveConfig, ListPermuteMoveConfig, ListPrecedenceMoveConfig,
    ListReverseMoveConfig, ListRuinMoveSelectorConfig, ListSwapMoveConfig, MoveSelectorConfig,
    NearbyListChangeMoveConfig, NearbyListSwapMoveConfig, SublistChangeMoveConfig,
    SublistSwapMoveConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    DynamicListAccess, DynamicListAccessCapabilities, DynamicListMetadata,
    DynamicListMetadataCapabilities, DynamicListVariableSlot, EntityClassId,
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor, VariableId,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use super::super::super::super::graph::ListLeafKind;
use super::super::{RuntimeListNeighborhoodPlan, RuntimeListNeighborhoodSelector};
pub(super) use super::clone_tracking::PositionMetric;
use crate::builder::context::RuntimeListSlot;
use crate::builder::ListVariableSlot;

pub(super) type Slot = RuntimeListSlot<ListPlan, usize, PositionMetric, PositionMetric>;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ListPlan {
    score: Option<SoftScore>,
    pub(super) elements: Vec<usize>,
    pub(super) routes: Vec<Vec<usize>>,
}
impl Clone for ListPlan {
    fn clone(&self) -> Self {
        super::clone_tracking::record_solution_clone();
        Self {
            score: self.score,
            elements: self.elements.clone(),
            routes: self.routes.clone(),
        }
    }
}
impl PlanningSolution for ListPlan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

pub(super) fn initial_plan() -> ListPlan {
    ListPlan {
        score: None,
        elements: (0..8).collect(),
        routes: vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7]],
    }
}

pub(super) fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("ListPlan", TypeId::of::<ListPlan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vec<usize>>(), "vehicles")
            .with_logical_id(EntityClassId(0))
            .with_extractor(Box::new(EntityCollectionExtractor::new(
                "Vehicle",
                "vehicles",
                |plan: &ListPlan| &plan.routes,
                |plan: &mut ListPlan| &mut plan.routes,
            )))
            .with_variable(VariableDescriptor::list("visits").with_logical_id(VariableId(0))),
    )
}

pub(super) fn director(plan: ListPlan) -> ScoreDirector<ListPlan, ()> {
    ScoreDirector::simple(plan, descriptor(), |plan, descriptor_index| {
        usize::from(descriptor_index == 0) * plan.routes.len()
    })
}

fn element_count(plan: &ListPlan) -> usize {
    plan.elements.len()
}

fn assigned_elements(plan: &ListPlan) -> Vec<usize> {
    plan.routes.iter().flatten().copied().collect()
}

fn entity_count(plan: &ListPlan) -> usize {
    plan.routes.len()
}

pub(super) fn list_len(plan: &ListPlan, entity: usize) -> usize {
    plan.routes[entity].len()
}

pub(super) fn list_get(plan: &ListPlan, entity: usize, position: usize) -> Option<usize> {
    plan.routes.get(entity)?.get(position).copied()
}

fn list_insert(plan: &mut ListPlan, entity: usize, position: usize, value: usize) {
    plan.routes[entity].insert(position, value);
}

fn list_remove(plan: &mut ListPlan, entity: usize, position: usize) -> Option<usize> {
    (position < plan.routes[entity].len()).then(|| plan.routes[entity].remove(position))
}

fn construction_remove(plan: &mut ListPlan, entity: usize, position: usize) -> usize {
    plan.routes[entity].remove(position)
}

fn list_set(plan: &mut ListPlan, entity: usize, position: usize, value: usize) {
    plan.routes[entity][position] = value;
}

fn list_reverse(plan: &mut ListPlan, entity: usize, start: usize, end: usize) {
    plan.routes[entity][start..end].reverse();
}

fn sublist_remove(plan: &mut ListPlan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.routes[entity].drain(start..end).collect()
}

fn sublist_insert(plan: &mut ListPlan, entity: usize, position: usize, values: Vec<usize>) {
    plan.routes[entity].splice(position..position, values);
}

fn index_to_element(plan: &ListPlan, index: usize) -> usize {
    plan.elements[index]
}

fn element_source_key(_: &ListPlan, element: &usize) -> usize {
    *element
}

fn precedence_duration(_: &ListPlan, _: usize) -> usize {
    1
}

fn precedence_successors(_: &ListPlan, element: usize, successors: &mut Vec<usize>) {
    if element % 4 != 3 {
        successors.push(element + 1);
    }
}

pub(super) fn distance(
    from_entity: usize,
    from_position: usize,
    to_entity: usize,
    to_position: usize,
) -> i64 {
    (from_entity.abs_diff(to_entity) as i64) * 100 + from_position.abs_diff(to_position) as i64
}

pub(super) fn native_slot() -> ListVariableSlot<ListPlan, usize, PositionMetric, PositionMetric> {
    ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        construction_remove,
        list_insert,
        index_to_element,
        element_source_key,
        entity_count,
        PositionMetric,
        PositionMetric,
        "visits",
        0,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
}

fn precedence_native_slot() -> ListVariableSlot<ListPlan, usize, PositionMetric, PositionMetric> {
    native_slot().with_precedence_hooks(Some(precedence_duration), Some(precedence_successors))
}

pub(super) fn static_slot() -> Slot {
    Slot::from_static(native_slot(), 0)
}

pub(super) fn static_slot_for(kind: ListLeafKind) -> Slot {
    if kind == ListLeafKind::Precedence {
        Slot::from_static(precedence_native_slot(), 0)
    } else {
        static_slot()
    }
}

#[derive(Debug)]
struct DynamicAccess;

impl DynamicListAccess<ListPlan> for DynamicAccess {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn entity_count(&self, plan: &ListPlan) -> usize {
        entity_count(plan)
    }

    fn element_count(&self, plan: &ListPlan) -> usize {
        element_count(plan)
    }

    fn element(&self, plan: &ListPlan, index: usize) -> Option<usize> {
        plan.elements.get(index).copied()
    }

    fn assigned_elements(&self, plan: &ListPlan) -> Vec<usize> {
        assigned_elements(plan)
    }

    fn len(&self, plan: &ListPlan, entity: usize) -> usize {
        list_len(plan, entity)
    }

    fn get(&self, plan: &ListPlan, entity: usize, position: usize) -> Option<usize> {
        list_get(plan, entity, position)
    }

    fn insert(&self, plan: &mut ListPlan, entity: usize, position: usize, value: usize) {
        list_insert(plan, entity, position, value);
    }

    fn remove(&self, plan: &mut ListPlan, entity: usize, position: usize) -> Option<usize> {
        list_remove(plan, entity, position)
    }

    fn capabilities(&self) -> DynamicListAccessCapabilities {
        DynamicListAccessCapabilities {
            set: true,
            reverse: true,
            sublist: true,
            ..DynamicListAccessCapabilities::default()
        }
    }

    fn set(&self, plan: &mut ListPlan, entity: usize, position: usize, value: usize) -> bool {
        list_set(plan, entity, position, value);
        true
    }

    fn reverse(&self, plan: &mut ListPlan, entity: usize, start: usize, end: usize) -> bool {
        list_reverse(plan, entity, start, end);
        true
    }

    fn sublist_remove(
        &self,
        plan: &mut ListPlan,
        entity: usize,
        start: usize,
        end: usize,
    ) -> Option<Vec<usize>> {
        Some(sublist_remove(plan, entity, start, end))
    }

    fn sublist_insert(
        &self,
        plan: &mut ListPlan,
        entity: usize,
        position: usize,
        values: Vec<usize>,
    ) -> bool {
        sublist_insert(plan, entity, position, values);
        true
    }
}

#[derive(Debug)]
struct DynamicMetadata {
    precedence: bool,
}

impl DynamicListMetadata<ListPlan> for DynamicMetadata {
    fn entity_class(&self) -> EntityClassId {
        EntityClassId(0)
    }

    fn variable(&self) -> VariableId {
        VariableId(0)
    }

    fn capabilities(&self) -> DynamicListMetadataCapabilities {
        DynamicListMetadataCapabilities {
            precedence_duration: self.precedence,
            precedence_successors: self.precedence,
            cross_position_distance: true,
            intra_position_distance: true,
            ..DynamicListMetadataCapabilities::default()
        }
    }

    fn element_owner(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn construction_order_key(&self, _: &ListPlan, _: usize) -> Option<i64> {
        None
    }

    fn precedence_duration(&self, _: &ListPlan, _: usize) -> Option<usize> {
        self.precedence.then_some(1)
    }

    fn extend_precedence_successors(
        &self,
        _: &ListPlan,
        element: usize,
        successors: &mut Vec<usize>,
    ) -> bool {
        if !self.precedence {
            return false;
        }
        if element % 4 != 3 {
            successors.push(element + 1);
        }
        true
    }

    fn cross_position_distance(
        &self,
        _: &ListPlan,
        from_entity: usize,
        from_position: usize,
        to_entity: usize,
        to_position: usize,
    ) -> Option<f64> {
        Some(distance(from_entity, from_position, to_entity, to_position) as f64)
    }

    fn intra_position_distance(
        &self,
        _: &ListPlan,
        entity: usize,
        from_position: usize,
        to_position: usize,
    ) -> Option<f64> {
        Some(distance(entity, from_position, entity, to_position) as f64)
    }

    fn route_depot(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn route_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn route_feasible(&self, _: &ListPlan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }

    fn savings_depot(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn savings_metric_class(&self, _: &ListPlan, _: usize) -> Option<usize> {
        None
    }

    fn savings_distance(&self, _: &ListPlan, _: usize, _: usize, _: usize) -> Option<i64> {
        None
    }

    fn savings_feasible(&self, _: &ListPlan, _: usize, _: &[usize]) -> Option<bool> {
        None
    }
}

fn dynamic_variable_slot_with_precedence(precedence: bool) -> DynamicListVariableSlot<ListPlan> {
    DynamicListVariableSlot::with_access_and_metadata(
        EntityClassId(0),
        VariableId(0),
        "Vehicle",
        "visits",
        Arc::new(DynamicAccess),
        Arc::new(DynamicMetadata { precedence }),
    )
    .expect("dynamic test access and metadata have matching identities")
    .resolved_against(&descriptor())
    .expect("dynamic test slot resolves against the list descriptor")
}

pub(super) fn dynamic_variable_slot() -> DynamicListVariableSlot<ListPlan> {
    dynamic_variable_slot_with_precedence(false)
}

pub(super) fn dynamic_slot() -> Slot {
    Slot::from_dynamic(dynamic_variable_slot())
}

pub(super) fn dynamic_slot_for(kind: ListLeafKind) -> Slot {
    Slot::from_dynamic(dynamic_variable_slot_with_precedence(
        kind == ListLeafKind::Precedence,
    ))
}

pub(super) fn config(kind: ListLeafKind) -> MoveSelectorConfig {
    let target = VariableTargetConfig::default();
    match kind {
        ListLeafKind::Change => MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
            selection_order: None,
            selection_metric: None,
            target,
        }),
        ListLeafKind::NearbyChange => {
            MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
                selection_order: None,
                selection_metric: None,
                max_nearby: 2,
                target,
            })
        }
        ListLeafKind::Swap => MoveSelectorConfig::ListSwapMoveSelector(ListSwapMoveConfig {
            selection_order: None,
            selection_metric: None,
            target,
        }),
        ListLeafKind::Permute => {
            MoveSelectorConfig::ListPermuteMoveSelector(ListPermuteMoveConfig {
                selection_order: None,
                selection_metric: None,
                min_window_size: 2,
                max_window_size: 3,
                target,
            })
        }
        ListLeafKind::Precedence => {
            MoveSelectorConfig::ListPrecedenceMoveSelector(ListPrecedenceMoveConfig {
                selection_order: None,
                selection_metric: None,
                target,
            })
        }
        ListLeafKind::NearbySwap => {
            MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
                selection_order: None,
                selection_metric: None,
                max_nearby: 2,
                target,
            })
        }
        ListLeafKind::SublistChange => {
            MoveSelectorConfig::SublistChangeMoveSelector(SublistChangeMoveConfig {
                selection_order: None,
                selection_metric: None,
                min_sublist_size: 1,
                max_sublist_size: 2,
                target,
            })
        }
        ListLeafKind::SublistSwap => {
            MoveSelectorConfig::SublistSwapMoveSelector(SublistSwapMoveConfig {
                selection_order: None,
                selection_metric: None,
                min_sublist_size: 1,
                max_sublist_size: 2,
                target,
            })
        }
        ListLeafKind::Reverse => {
            MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig {
                selection_order: None,
                selection_metric: None,
                target,
            })
        }
        ListLeafKind::KOpt => MoveSelectorConfig::KOptMoveSelector(KOptMoveSelectorConfig {
            selection_order: None,
            selection_metric: None,
            k: 2,
            min_segment_len: 1,
            max_nearby: 0,
            target,
        }),
        ListLeafKind::Ruin => {
            MoveSelectorConfig::ListRuinMoveSelector(ListRuinMoveSelectorConfig {
                selection_order: None,
                selection_metric: None,
                min_ruin_count: 1,
                max_ruin_count: 2,
                moves_per_step: Some(3),
                max_source_list_len: None,
                skip_empty_destinations: false,
                target,
            })
        }
    }
}

pub(super) fn selector(
    kind: ListLeafKind,
    slot: Slot,
    seed: Option<u64>,
) -> RuntimeListNeighborhoodSelector<ListPlan, usize, PositionMetric, PositionMetric> {
    RuntimeListNeighborhoodSelector::new(
        RuntimeListNeighborhoodPlan::from_compiled(kind, &config(kind), vec![slot], seed)
            .expect("test list plan compiles"),
    )
}

pub(super) const ALL_KINDS: [ListLeafKind; 11] = [
    ListLeafKind::Change,
    ListLeafKind::NearbyChange,
    ListLeafKind::Swap,
    ListLeafKind::Permute,
    ListLeafKind::Precedence,
    ListLeafKind::NearbySwap,
    ListLeafKind::SublistChange,
    ListLeafKind::SublistSwap,
    ListLeafKind::Reverse,
    ListLeafKind::KOpt,
    ListLeafKind::Ruin,
];
