use super::*;
use std::any::TypeId;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use solverforge_config::{
    CartesianProductConfig, ListChangeMoveConfig, ListPermuteMoveConfig, ListReverseMoveConfig,
    ListSwapMoveConfig, MoveSelectorConfig, NearbyListChangeMoveConfig, NearbyListSwapMoveConfig,
    SublistChangeMoveConfig, SublistSwapMoveConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

use crate::builder::ListVariableSlot;
use crate::heuristic::r#move::ListMoveUnion;
use crate::heuristic::selector::move_selector::{
    collect_cursor_indices, MoveCandidateRef, MoveCursor, MoveSelector,
};
use crate::CrossEntityDistanceMeter;

#[derive(Clone, Debug)]
struct Vehicle {
    visits: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Plan {
    vehicles: Vec<Vehicle>,
    score: Option<SoftScore>,
}

impl PlanningSolution for Plan {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[derive(Clone, Debug)]
struct CountingMeter {
    calls: Arc<AtomicUsize>,
}

impl CountingMeter {
    fn new() -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                calls: calls.clone(),
            },
            calls,
        )
    }
}

impl CrossEntityDistanceMeter<Plan> for CountingMeter {
    fn distance(
        &self,
        _solution: &Plan,
        _src_entity: usize,
        _src_pos: usize,
        _dst_entity: usize,
        _dst_pos: usize,
    ) -> f64 {
        self.calls.fetch_add(1, Ordering::SeqCst);
        1.0
    }
}

fn descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(
        EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles").with_extractor(
            Box::new(EntityCollectionExtractor::new(
                "Vehicle",
                "vehicles",
                |s: &Plan| &s.vehicles,
                |s: &mut Plan| &mut s.vehicles,
            )),
        ),
    )
}

fn list_len(s: &Plan, entity_idx: usize) -> usize {
    s.vehicles
        .get(entity_idx)
        .map_or(0, |vehicle| vehicle.visits.len())
}

fn list_remove(s: &mut Plan, entity_idx: usize, pos: usize) -> Option<usize> {
    let visits = &mut s.vehicles.get_mut(entity_idx)?.visits;
    if pos < visits.len() {
        Some(visits.remove(pos))
    } else {
        None
    }
}

fn list_insert(s: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
    if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
        vehicle.visits.insert(pos, value);
    }
}

fn list_get(s: &Plan, entity_idx: usize, pos: usize) -> Option<usize> {
    s.vehicles
        .get(entity_idx)
        .and_then(|vehicle| vehicle.visits.get(pos))
        .copied()
}

fn list_set(s: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
    if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
        vehicle.visits[pos] = value;
    }
}

fn list_reverse(s: &mut Plan, entity_idx: usize, start: usize, end: usize) {
    if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
        vehicle.visits[start..end].reverse();
    }
}

fn sublist_remove(s: &mut Plan, entity_idx: usize, start: usize, end: usize) -> Vec<usize> {
    if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
        vehicle.visits.drain(start..end).collect()
    } else {
        Vec::new()
    }
}

fn sublist_insert(s: &mut Plan, entity_idx: usize, pos: usize, values: Vec<usize>) {
    if let Some(vehicle) = s.vehicles.get_mut(entity_idx) {
        vehicle.visits.splice(pos..pos, values);
    }
}

fn ruin_remove(s: &mut Plan, entity_idx: usize, pos: usize) -> usize {
    s.vehicles[entity_idx].visits.remove(pos)
}

fn ruin_insert(s: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
    s.vehicles[entity_idx].visits.insert(pos, value);
}

fn entity_count(s: &Plan) -> usize {
    s.vehicles.len()
}

fn element_count(s: &Plan) -> usize {
    s.vehicles.iter().map(|vehicle| vehicle.visits.len()).sum()
}

fn assigned_elements(s: &Plan) -> Vec<usize> {
    s.vehicles
        .iter()
        .flat_map(|vehicle| vehicle.visits.iter().copied())
        .collect()
}

fn construction_list_remove(s: &mut Plan, entity_idx: usize, pos: usize) -> usize {
    s.vehicles[entity_idx].visits.remove(pos)
}

fn index_to_element(s: &Plan, idx: usize) -> usize {
    assigned_elements(s).get(idx).copied().unwrap_or(idx)
}

fn precedence_duration(_s: &Plan, _element: usize) -> usize {
    1
}

fn precedence_successors(_s: &Plan, element: usize, out: &mut Vec<usize>) {
    if element == 0 {
        out.push(1);
    }
}

fn list_slot_with_precedence() -> ListVariableSlot<Plan, usize, CountingMeter, CountingMeter> {
    let (cross_meter, _) = CountingMeter::new();
    let (intra_meter, _) = CountingMeter::new();
    ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        entity_count,
        cross_meter,
        intra_meter,
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
    .with_precedence_hooks(Some(precedence_duration), Some(precedence_successors))
}

#[test]
fn nearby_list_swap_uses_cross_entity_meter() {
    let (cross_meter, cross_calls) = CountingMeter::new();
    let (intra_meter, intra_calls) = CountingMeter::new();
    let ctx = ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        entity_count,
        cross_meter,
        intra_meter,
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
    );
    let config = MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
        max_nearby: 4,
        target: VariableTargetConfig::default(),
    });
    let selector = ListMoveSelectorBuilder::build(Some(&config), &ctx, None);
    let solution = Plan {
        vehicles: vec![Vehicle { visits: vec![10] }, Vehicle { visits: vec![20] }],
        score: None,
    };
    let director = ScoreDirector::simple(solution, descriptor(), |s, descriptor_index| {
        if descriptor_index == 0 {
            s.vehicles.len()
        } else {
            0
        }
    });

    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(moves.len(), 1, "expected a single inter-entity swap");
    assert!(
        cross_calls.load(Ordering::SeqCst) > 0,
        "nearby_list_swap must evaluate distances through the cross-entity meter"
    );
    assert_eq!(
        intra_calls.load(Ordering::SeqCst),
        0,
        "nearby_list_swap must not consult the intra-route meter"
    );
}

#[test]
fn precedence_route_hooks_filter_cyclic_same_route_leaf_moves() {
    let ctx = list_slot_with_precedence();
    let director = ScoreDirector::simple(
        Plan {
            vehicles: vec![Vehicle {
                visits: vec![0, 1, 2],
            }],
            score: None,
        },
        descriptor(),
        |s, descriptor_index| {
            if descriptor_index == 0 {
                s.vehicles.len()
            } else {
                0
            }
        },
    );

    let move_configs = [
        MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig::default()),
        MoveSelectorConfig::NearbyListChangeMoveSelector(NearbyListChangeMoveConfig {
            max_nearby: 8,
            target: VariableTargetConfig::default(),
        }),
        MoveSelectorConfig::ListSwapMoveSelector(ListSwapMoveConfig::default()),
        MoveSelectorConfig::NearbyListSwapMoveSelector(NearbyListSwapMoveConfig {
            max_nearby: 8,
            target: VariableTargetConfig::default(),
        }),
        MoveSelectorConfig::ListReverseMoveSelector(ListReverseMoveConfig::default()),
        MoveSelectorConfig::ListPermuteMoveSelector(ListPermuteMoveConfig {
            min_window_size: 2,
            max_window_size: 2,
            target: VariableTargetConfig::default(),
        }),
        MoveSelectorConfig::SublistChangeMoveSelector(SublistChangeMoveConfig {
            min_sublist_size: 1,
            max_sublist_size: 1,
            target: VariableTargetConfig::default(),
        }),
        MoveSelectorConfig::SublistSwapMoveSelector(SublistSwapMoveConfig {
            min_sublist_size: 1,
            max_sublist_size: 1,
            target: VariableTargetConfig::default(),
        }),
    ];

    for config in move_configs {
        let selector = ListMoveSelectorBuilder::build(Some(&config), &ctx, None);
        let moves = selector.iter_moves(&director).collect::<Vec<_>>();
        assert_eq!(
            selector.size(&director),
            moves.len(),
            "guarded selector size must match streamed candidates for {config:?}",
        );
        assert!(
            !moves.iter().any(is_cyclic_same_route_move),
            "precedence route guard must filter cyclic same-route moves for {config:?}: {moves:?}",
        );
    }
}

fn is_cyclic_same_route_move(mov: &ListMoveUnion<Plan, usize>) -> bool {
    match mov {
        ListMoveUnion::ListChange(mov) => {
            mov.source_entity_index() == 0
                && mov.dest_entity_index() == 0
                && ((mov.source_position(), mov.dest_position()) == (0, 2)
                    || (mov.source_position(), mov.dest_position()) == (1, 0))
        }
        ListMoveUnion::ListSwap(mov) => {
            mov.first_entity_index() == 0
                && mov.second_entity_index() == 0
                && mov.first_position() == 0
                && mov.second_position() == 1
        }
        ListMoveUnion::ListReverse(mov) => {
            mov.entity_index() == 0 && mov.start() == 0 && mov.end() == 2
        }
        ListMoveUnion::ListPermute(mov) => {
            mov.entity_index() == 0
                && mov.start() == 0
                && mov.end() == 2
                && mov.permutation() == [1, 0]
        }
        ListMoveUnion::SublistChange(mov) => {
            mov.source_entity_index() == 0
                && mov.dest_entity_index() == 0
                && mov.source_start() == 0
                && mov.source_end() == 1
                && mov.dest_position() == 1
        }
        ListMoveUnion::SublistSwap(mov) => {
            mov.first_entity_index() == 0
                && mov.second_entity_index() == 0
                && mov.first_start() == 0
                && mov.first_end() == 1
                && mov.second_start() == 1
                && mov.second_end() == 2
        }
        _ => false,
    }
}

#[test]
fn public_list_builder_supports_cartesian_product() {
    let (cross_meter, _) = CountingMeter::new();
    let (intra_meter, _) = CountingMeter::new();
    let ctx = ListVariableSlot::new(
        "Vehicle",
        element_count,
        assigned_elements,
        list_len,
        list_remove,
        construction_list_remove,
        list_insert,
        list_get,
        list_set,
        list_reverse,
        sublist_remove,
        sublist_insert,
        ruin_remove,
        ruin_insert,
        index_to_element,
        entity_count,
        cross_meter,
        intra_meter,
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
    );
    let config = MoveSelectorConfig::CartesianProductMoveSelector(CartesianProductConfig {
        require_hard_improvement: false,
        selectors: vec![
            MoveSelectorConfig::ListChangeMoveSelector(ListChangeMoveConfig {
                target: VariableTargetConfig::default(),
            }),
            MoveSelectorConfig::ListSwapMoveSelector(ListSwapMoveConfig {
                target: VariableTargetConfig::default(),
            }),
        ],
    });
    let selector = ListMoveSelectorBuilder::build(Some(&config), &ctx, None);
    let solution = Plan {
        vehicles: vec![Vehicle { visits: vec![10] }, Vehicle { visits: vec![20] }],
        score: None,
    };
    let director = ScoreDirector::simple(solution, descriptor(), |s, descriptor_index| {
        if descriptor_index == 0 {
            s.vehicles.len()
        } else {
            0
        }
    });

    let mut cursor = selector.open_cursor(&director);
    let indices =
        collect_cursor_indices::<Plan, crate::heuristic::r#move::ListMoveUnion<Plan, usize>, _>(
            &mut cursor,
        );

    assert!(!indices.is_empty());
    assert!(indices.iter().all(|&index| matches!(
        cursor.candidate(index),
        Some(MoveCandidateRef::Sequential(_))
    )));
    assert!(matches!(
        cursor.candidate(indices[0]),
        Some(MoveCandidateRef::Sequential(_))
    ));
}
