use std::any::TypeId;
use std::hint::black_box;
use std::time::Instant;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::list_change::ListChangeMoveSelector;
use crate::heuristic::selector::list_swap::ListSwapMoveSelector;
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

fn list_remove(plan: &mut Plan, entity_idx: usize, pos: usize) -> Option<usize> {
    plan.vehicles
        .get_mut(entity_idx)
        .and_then(|vehicle| (pos < vehicle.visits.len()).then(|| vehicle.visits.remove(pos)))
}

fn list_insert(plan: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity_idx) {
        vehicle.visits.insert(pos, value);
    }
}

fn list_get(plan: &Plan, entity_idx: usize, pos: usize) -> Option<usize> {
    plan.vehicles
        .get(entity_idx)
        .and_then(|vehicle| vehicle.visits.get(pos))
        .copied()
}

fn list_set(plan: &mut Plan, entity_idx: usize, pos: usize, value: usize) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity_idx) {
        vehicle.visits[pos] = value;
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
fn list_change_keeps_canonical_intra_then_inter_order() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![10, 20],
        },
        Vehicle { visits: vec![30] },
    ]);

    let selector = ListChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    let moves: Vec<_> = selector
        .iter_moves(&director)
        .map(|m| {
            (
                m.source_entity_index(),
                m.source_position(),
                m.dest_entity_index(),
                m.dest_position(),
            )
        })
        .collect();

    assert_eq!(
        moves,
        vec![
            (0, 0, 1, 0),
            (0, 0, 1, 1),
            (0, 1, 0, 0),
            (0, 1, 1, 0),
            (0, 1, 1, 1),
            (1, 0, 0, 0),
            (1, 0, 0, 1),
            (1, 0, 0, 2),
        ],
        "list_change must enumerate intra-list moves first, then cross-entity insertions"
    );
}

#[test]
fn list_change_moves_are_doable() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![1, 2, 3],
        },
        Vehicle { visits: vec![10] },
    ]);

    let selector = ListChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    for move_candidate in selector.iter_moves(&director) {
        assert!(
            move_candidate.is_doable(&director),
            "generated list_change move should be doable: {move_candidate:?}"
        );
    }
}

#[test]
fn list_swap_emits_unique_pairs_in_canonical_order() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![10, 20],
        },
        Vehicle {
            visits: vec![30, 40],
        },
    ]);

    let selector = ListSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );

    let moves: Vec<_> = selector
        .iter_moves(&director)
        .map(|m| {
            (
                m.first_entity_index(),
                m.first_position(),
                m.second_entity_index(),
                m.second_position(),
            )
        })
        .collect();

    assert_eq!(
        moves,
        vec![
            (0, 0, 0, 1),
            (0, 0, 1, 0),
            (0, 0, 1, 1),
            (0, 1, 1, 0),
            (0, 1, 1, 1),
            (1, 0, 1, 1),
        ],
        "list_swap must preserve canonical pair order without reverse duplicates"
    );
}

#[test]
fn list_swap_moves_are_doable() {
    let director = create_director(vec![
        Vehicle {
            visits: vec![1, 2, 3],
        },
        Vehicle {
            visits: vec![10, 20, 30],
        },
    ]);

    let selector = ListSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );

    for move_candidate in selector.iter_moves(&director) {
        assert!(
            move_candidate.is_doable(&director),
            "generated list_swap move should be doable: {move_candidate:?}"
        );
    }
}

#[test]
fn bench_list_change_cursor_enumeration() {
    let director = create_director(benchmark_plan(12, 24));
    let selector = ListChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_remove,
        list_insert,
        "visits",
        0,
    );

    benchmark_cursor("list_change_cursor", 15, || {
        selector.iter_moves(&director).count()
    });
}

#[test]
fn bench_list_swap_cursor_enumeration() {
    let director = create_director(benchmark_plan(12, 24));
    let selector = ListSwapMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        list_set,
        "visits",
        0,
    );

    benchmark_cursor("list_swap_cursor", 15, || {
        selector.iter_moves(&director).count()
    });
}
