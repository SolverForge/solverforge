use std::alloc::{GlobalAlloc, Layout, System};
use std::any::TypeId;
use std::hint::black_box;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use solverforge_solver::heuristic::r#move::ChangeMove;
use solverforge_solver::heuristic::r#move::Move;
use solverforge_solver::heuristic::selector::decorator::{
    CartesianProductSelector, FilteringMoveSelector,
};
use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
use solverforge_solver::heuristic::selector::k_opt::{KOptConfig, KOptMoveSelector};
use solverforge_solver::heuristic::selector::list_change::ListChangeMoveSelector;
use solverforge_solver::heuristic::selector::list_swap::ListSwapMoveSelector;
use solverforge_solver::heuristic::selector::move_selector::{MoveCandidateRef, MoveCursor};
use solverforge_solver::heuristic::selector::nearby_list_change::{
    CrossEntityDistanceMeter, NearbyListChangeMoveSelector,
};
use solverforge_solver::heuristic::selector::nearby_list_swap::NearbyListSwapMoveSelector;
use solverforge_solver::heuristic::selector::sublist_change::SublistChangeMoveSelector;
use solverforge_solver::heuristic::selector::sublist_swap::SublistSwapMoveSelector;
use solverforge_solver::heuristic::selector::{
    ChangeMoveSelector, MoveSelector, RuinMoveSelector, RuinVariableAccess, StaticValueSelector,
};
use solverforge_solver::phase::Phase;
#[cfg(not(feature = "candidate"))]
use solverforge_solver::EntityPlacer;
use solverforge_solver::{
    ConstructionHeuristicPhase, FirstFitForager, QueuedEntityPlacer, SolverScope,
};
#[cfg(feature = "candidate")]
use solverforge_solver::{EntityPlacer, EntityPlacerCursor};

struct MeasuringAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);
static LIVE_BYTES: AtomicIsize = AtomicIsize::new(0);
static PEAK_LIVE_BYTES: AtomicIsize = AtomicIsize::new(0);
static ITERATIONS: OnceLock<usize> = OnceLock::new();

#[global_allocator]
static GLOBAL_ALLOCATOR: MeasuringAllocator = MeasuringAllocator;

unsafe impl GlobalAlloc for MeasuringAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record_allocation(layout.size());
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as isize, Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        let replacement = unsafe { System.realloc(pointer, old, new_size) };
        if !replacement.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size, Ordering::Relaxed);
            let delta = new_size as isize - old.size() as isize;
            let live = LIVE_BYTES.fetch_add(delta, Ordering::Relaxed) + delta;
            update_peak(live);
        }
        replacement
    }
}

fn record_allocation(size: usize) {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    ALLOCATED_BYTES.fetch_add(size, Ordering::Relaxed);
    let live = LIVE_BYTES.fetch_add(size as isize, Ordering::Relaxed) + size as isize;
    update_peak(live);
}

fn update_peak(live: isize) {
    let mut peak = PEAK_LIVE_BYTES.load(Ordering::Relaxed);
    while live > peak {
        match PEAK_LIVE_BYTES.compare_exchange_weak(
            peak,
            live,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(current) => peak = current,
        }
    }
}

fn reset_allocator_metrics() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    LIVE_BYTES.store(0, Ordering::Relaxed);
    PEAK_LIVE_BYTES.store(0, Ordering::Relaxed);
}

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

#[derive(Clone, Copy, Debug, Default)]
struct PositionDistanceMeter;

impl CrossEntityDistanceMeter<Plan> for PositionDistanceMeter {
    fn distance(
        &self,
        _solution: &Plan,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> f64 {
        source_entity.abs_diff(destination_entity) as f64 * 100.0
            + source_position.abs_diff(destination_position) as f64
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
    let entity = EntityDescriptor::new("Vehicle", TypeId::of::<Vehicle>(), "vehicles")
        .with_extractor(extractor);
    SolutionDescriptor::new("Plan", TypeId::of::<Plan>()).with_entity(entity)
}

fn director(vehicle_count: usize, visits_per_vehicle: usize) -> ScoreDirector<Plan, ()> {
    let vehicles = (0..vehicle_count)
        .map(|vehicle| Vehicle {
            visits: (0..visits_per_vehicle)
                .map(|visit| vehicle * 1_000 + visit)
                .collect(),
        })
        .collect();
    ScoreDirector::simple(
        Plan {
            vehicles,
            score: None,
        },
        descriptor(),
        |plan, _| plan.vehicles.len(),
    )
}

fn list_len(plan: &Plan, entity: usize) -> usize {
    plan.vehicles
        .get(entity)
        .map_or(0, |vehicle| vehicle.visits.len())
}

fn list_get(plan: &Plan, entity: usize, position: usize) -> Option<usize> {
    plan.vehicles
        .get(entity)
        .and_then(|vehicle| vehicle.visits.get(position))
        .copied()
}

fn list_set(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity) {
        vehicle.visits[position] = value;
    }
}

fn list_remove(plan: &mut Plan, entity: usize, position: usize) -> Option<usize> {
    plan.vehicles.get_mut(entity).and_then(|vehicle| {
        (position < vehicle.visits.len()).then(|| vehicle.visits.remove(position))
    })
}

fn list_insert(plan: &mut Plan, entity: usize, position: usize, value: usize) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity) {
        vehicle.visits.insert(position, value);
    }
}

fn sublist_remove(plan: &mut Plan, entity: usize, start: usize, end: usize) -> Vec<usize> {
    plan.vehicles
        .get_mut(entity)
        .map(|vehicle| vehicle.visits.drain(start..end).collect())
        .unwrap_or_default()
}

fn sublist_insert(plan: &mut Plan, entity: usize, position: usize, values: Vec<usize>) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity) {
        for (offset, value) in values.into_iter().enumerate() {
            vehicle.visits.insert(position + offset, value);
        }
    }
}

fn mix(hash: &mut u64, value: usize) {
    *hash ^= value as u64;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
}

fn mix_u64(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
}

fn mix_move_identity(hash: &mut u64, identity: &[u64]) {
    for &component in identity {
        mix_u64(hash, component);
    }
}

fn iterations() -> usize {
    *ITERATIONS.get().expect("benchmark iterations must be set")
}

fn emit(case: &str, expected_count: usize, run: impl FnOnce() -> (usize, u64)) {
    reset_allocator_metrics();
    let started = Instant::now();
    let (count, order_hash) = run();
    let elapsed = started.elapsed();
    assert_eq!(
        count,
        expected_count * iterations(),
        "{case} candidate count changed"
    );
    println!(
        "{{\"case\":\"{case}\",\"iterations\":{},\"candidate_count\":{expected_count},\"total_candidate_count\":{count},\"order_hash\":{order_hash},\"wall_ns\":{},\"allocations\":{},\"allocated_bytes\":{},\"peak_live_bytes\":{}}}",
        iterations(),
        elapsed.as_nanos(),
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
        PEAK_LIVE_BYTES.load(Ordering::Relaxed).max(0),
    );
}

include!("selector_cursor_gate/list_cases.rs");

fn scalar_get(plan: &Plan, entity: usize, _variable_index: usize) -> Option<usize> {
    plan.vehicles
        .get(entity)
        .and_then(|vehicle| vehicle.visits.first())
        .copied()
}

fn scalar_set(plan: &mut Plan, entity: usize, _variable_index: usize, value: Option<usize>) {
    if let Some(vehicle) = plan.vehicles.get_mut(entity) {
        match (vehicle.visits.first_mut(), value) {
            (Some(first), Some(value)) => *first = value,
            (None, Some(value)) => vehicle.visits.push(value),
            (Some(_), None) => {
                vehicle.visits.remove(0);
            }
            (None, None) => {}
        }
    }
}

fn unassigned_director(vehicle_count: usize) -> ScoreDirector<Plan, ()> {
    ScoreDirector::simple(
        Plan {
            vehicles: (0..vehicle_count)
                .map(|_| Vehicle { visits: Vec::new() })
                .collect(),
            score: None,
        },
        descriptor(),
        |plan, _| plan.vehicles.len(),
    )
}

fn keep_even_source(
    candidate: MoveCandidateRef<
        '_,
        Plan,
        solverforge_solver::heuristic::r#move::ListChangeMove<Plan, usize>,
    >,
) -> bool {
    matches!(candidate, MoveCandidateRef::Borrowed(mov) if mov.source_entity_index().is_multiple_of(2))
}

fn filtering() {
    let director = director(12, 24);
    let inner = ListChangeMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        list_remove,
        list_insert,
        "visits",
        0,
    );
    let selector = FilteringMoveSelector::new(inner, keep_even_source);
    emit("filtering", 42_912, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                mix(&mut hash, mov.source_entity_index());
                mix(&mut hash, mov.source_position());
                mix(&mut hash, mov.dest_entity_index());
                mix(&mut hash, mov.dest_position());
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn cartesian() {
    let director = director(12, 4);
    let left = ChangeMoveSelector::simple(scalar_get, scalar_set, 0, 0, "first", vec![10, 11]);
    let right = ChangeMoveSelector::simple(scalar_get, scalar_set, 0, 0, "first", vec![20, 21]);
    let selector = CartesianProductSelector::new(left, right);
    emit("cartesian", 576, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            let mut cursor = selector.open_cursor(&director);
            while let Some(candidate_id) = cursor.next_candidate() {
                let mov = cursor
                    .candidate(candidate_id)
                    .expect("cartesian benchmark candidate must remain live");
                let signature = mov.tabu_signature(&director);
                mix_move_identity(&mut hash, &signature.move_id);
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn k_opt() {
    let director = director(1, 18);
    let selector = KOptMoveSelector::<Plan, usize, _>::new(
        FromSolutionEntitySelector::new(0),
        KOptConfig::new(3),
        list_len,
        list_get,
        sublist_remove,
        sublist_insert,
        "visits",
        0,
    );
    emit("k_opt", 4_760, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                for cut in mov.cuts() {
                    mix(&mut hash, cut.entity_index());
                    mix(&mut hash, cut.position());
                }
                let signature = mov.tabu_signature(&director);
                mix_move_identity(&mut hash, &signature.move_id);
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

fn entity_count(plan: &Plan) -> usize {
    plan.vehicles.len()
}

fn ruin() {
    let director = director(4_096, 1);
    let selector = RuinMoveSelector::<Plan, usize>::new(
        4,
        8,
        RuinVariableAccess::new(entity_count, scalar_get, scalar_set, 0, "first", 0),
    )
    .with_moves_per_step(4_096)
    .with_seed(0x5EED);
    emit("ruin", 4_096, || {
        let mut count = 0;
        let mut hash = 0xCBF2_9CE4_8422_2325;
        for _ in 0..iterations() {
            for mov in selector.iter_moves(&director) {
                for &entity_index in mov.entity_indices_slice() {
                    mix(&mut hash, entity_index);
                }
                black_box(&mov);
                count += 1;
            }
        }
        (count, hash)
    });
}

include!("selector_cursor_gate/construction_cases.rs");

fn main() {
    let runtime_case = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    // The zero-regression gate supplies this at compile time so each selector is
    // measured in an independently linked executable. That prevents unrelated
    // selector code layout from perturbing branch-predictor and cache counters.
    let case = option_env!("SOLVERFORGE_BENCH_CASE").unwrap_or(runtime_case.as_str());
    let iteration_count = std::env::args()
        .nth(2)
        .map(|value| {
            value
                .parse::<usize>()
                .expect("iterations must be an integer")
        })
        .unwrap_or(16);
    assert!(iteration_count > 0, "iterations must be positive");
    ITERATIONS
        .set(iteration_count)
        .expect("benchmark iterations must only be set once");
    let run = |name: &str, function: fn()| {
        if case == "all" || case == name {
            function();
        }
    };
    run("list_change", list_change);
    run("list_swap", list_swap);
    run("nearby_change", nearby_change);
    run("nearby_swap", nearby_swap);
    run("sublist_change", sublist_change);
    run("sublist_swap", sublist_swap);
    run("filtering", filtering);
    run("cartesian", cartesian);
    run("k_opt", k_opt);
    run("ruin", ruin);
    run("construction_full", construction_full);
    run("construction_first_fit", construction_first_fit);
    assert!(
        case == "all"
            || matches!(
                case,
                "list_change"
                    | "list_swap"
                    | "nearby_change"
                    | "nearby_swap"
                    | "sublist_change"
                    | "sublist_swap"
                    | "filtering"
                    | "cartesian"
                    | "k_opt"
                    | "ruin"
                    | "construction_full"
                    | "construction_first_fit"
            ),
        "unknown selector benchmark case `{case}`"
    );
}
