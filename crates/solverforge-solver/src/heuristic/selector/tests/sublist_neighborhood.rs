use std::any::TypeId;
use std::hint::black_box;
use std::time::Instant;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::sublist_change::SublistChangeMoveSelector;
use crate::heuristic::selector::sublist_swap::SublistSwapMoveSelector;
use crate::heuristic::selector::MoveSelector;
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

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

fn get_vehicles(plan: &Plan) -> &Vec<Vehicle> {
    &plan.vehicles
}

fn get_vehicles_mut(plan: &mut Plan) -> &mut Vec<Vehicle> {
    &mut plan.vehicles
}

fn descriptor() -> SolutionDescriptor {
    let extractor = Box::new(EntityCollectionExtractor::new(
        "Vehicle",
        "vehicles",
        get_vehicles,
        get_vehicles_mut,
    ));
    let entity_desc = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
        .with_extractor(extractor);
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(entity_desc)
}

fn create_director(vehicles: Vec<Vehicle>) -> ScoreDirector<Plan, ()> {
    let solution = Plan {
        vehicles,
        score: None,
    };
    ScoreDirector::simple(solution, descriptor(), |plan, _| plan.vehicles.len())
}

fn list_len(plan: &Plan, entity_idx: usize) -> usize {
    plan.vehicles
        .get(entity_idx)
        .map_or(0, |vehicle| vehicle.visits.len())
}
fn list_get(plan: &Plan, entity_idx: usize, pos: usize) -> Option<usize> {
    plan.vehicles
        .get(entity_idx)
        .and_then(|vehicle| vehicle.visits.get(pos))
        .copied()
}

fn sublist_remove(plan: &mut Plan, entity_idx: usize, start: usize, end: usize) -> Vec<usize> {
    plan.vehicles
        .get_mut(entity_idx)
        .map(|vehicle| vehicle.visits.drain(start..end).collect())
        .unwrap_or_default()
}

fn sublist_insert(plan: &mut Plan, entity_idx: usize, pos: usize, items: Vec<usize>) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity_idx) {
        for (offset, item) in items.into_iter().enumerate() {
            vehicle.visits.insert(pos + offset, item);
        }
    }
}

fn benchmark_cursor(name: &str, runs: usize, mut run: impl FnMut() -> usize) {
    let mut samples = Vec::with_capacity(runs);
    let warmup_count = black_box(run());
    assert!(warmup_count > 0, "{name} benchmark must enumerate moves");

    for _ in 0..runs {
        let start = Instant::now();
        let move_count = black_box(run());
        samples.push((move_count, start.elapsed()));
    }

    samples.sort_by_key(|(_, elapsed)| *elapsed);
    let (median_count, median_elapsed) = samples[runs / 2];
    let throughput = median_count as f64 / median_elapsed.as_secs_f64();
    eprintln!(
        "{name}: {median_count} moves, median {:?}, {:.0} moves/sec",
        median_elapsed, throughput
    );
}

fn benchmark_plan(vehicle_count: usize, visits_per_vehicle: usize) -> Vec<Vehicle> {
    (0..vehicle_count)
        .map(|vehicle_idx| Vehicle {
            visits: (0..visits_per_vehicle)
                .map(|visit_idx| vehicle_idx * 1_000 + visit_idx)
                .collect(),
        })
        .collect()
}

#[test]
fn sublist_change_keeps_canonical_segment_order() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![10, 20, 30],
        },
        Vehicle {
            visits: vec![40, 50],
        },
    ]);

    let selector = SublistChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        2,
        2,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    let moves: Vec<_> = selector
        .iter_moves(&director)
        .map(|m| {
            (
                m.source_entity_index(),
                m.source_start(),
                m.source_end(),
                m.dest_entity_index(),
                m.dest_position(),
            )
        })
        .collect();

    assert_eq!(
        moves,
        vec![
            (0, 0, 2, 0, 1),
            (0, 0, 2, 1, 0),
            (0, 0, 2, 1, 1),
            (0, 0, 2, 1, 2),
            (0, 1, 3, 0, 0),
            (0, 1, 3, 1, 0),
            (0, 1, 3, 1, 1),
            (0, 1, 3, 1, 2),
            (1, 0, 2, 0, 0),
            (1, 0, 2, 0, 1),
            (1, 0, 2, 0, 2),
            (1, 0, 2, 0, 3),
        ],
        "sublist_change must enumerate source segments in route order, with intra moves before cross-entity insertions"
    );
    assert_eq!(selector.size(&director), moves.len());
}

#[test]
fn sublist_change_moves_are_doable() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![1, 2, 3, 4],
        },
        Vehicle {
            visits: vec![10, 20, 30],
        },
    ]);

    let selector = SublistChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    for move_candidate in selector.iter_moves(&director) {
        assert!(
            move_candidate.is_doable(&director),
            "generated sublist_change move should be doable: {move_candidate:?}"
        );
    }
}

#[test]
fn sublist_swap_emits_canonical_non_overlapping_pairs() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![10, 20, 30, 40],
        },
        Vehicle {
            visits: vec![50, 60, 70],
        },
    ]);

    let selector = SublistSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        2,
        2,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    let moves: Vec<_> = selector
        .iter_moves(&director)
        .map(|m| {
            (
                m.first_entity_index(),
                m.first_start(),
                m.first_end(),
                m.second_entity_index(),
                m.second_start(),
                m.second_end(),
            )
        })
        .collect();

    assert_eq!(
        moves,
        vec![
            (0, 0, 2, 0, 2, 4),
            (0, 0, 2, 1, 0, 2),
            (0, 0, 2, 1, 1, 3),
            (0, 1, 3, 1, 0, 2),
            (0, 1, 3, 1, 1, 3),
            (0, 2, 4, 1, 0, 2),
            (0, 2, 4, 1, 1, 3),
        ],
        "sublist_swap must keep canonical intra-first and entity-pair ordering without overlap duplicates"
    );
    assert_eq!(selector.size(&director), moves.len());
}

#[test]
fn sublist_swap_moves_are_doable() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![1, 2, 3, 4, 5],
        },
        Vehicle {
            visits: vec![10, 20, 30, 40],
        },
    ]);

    let selector = SublistSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    for move_candidate in selector.iter_moves(&director) {
        assert!(
            move_candidate.is_doable(&director),
            "generated sublist_swap move should be doable: {move_candidate:?}"
        );
    }
}

#[test]
fn bench_sublist_change_cursor_enumeration() {
    let director = create_director(benchmark_plan(10, 18));
    let selector = SublistChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    benchmark_cursor("sublist_change_cursor", 15, || {
        selector.iter_moves(&director).count()
    });
}

#[test]
fn bench_sublist_swap_cursor_enumeration() {
    let director = create_director(benchmark_plan(10, 18));
    let selector = SublistSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        1,
        3,
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );

    benchmark_cursor("sublist_swap_cursor", 15, || {
        selector.iter_moves(&director).count()
    });
}
