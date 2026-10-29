/* Zero-regression gate for the compiled runtime's own neighborhood and
construction cursors.

Every selector and construction case compiles a `SolverConfig` over a
`RuntimeModel` with `CompiledRuntimeRunner::compile`, the compilation,
preparation, and lowering a configured solve runs, and then pulls candidates
from the cursor that runner hands out: the lowered local-search composition
cursor for neighborhoods, the descriptor-placement placer for scalar
construction, and the runner itself as the phase for first-fit construction.
No case builds a selector, placer, or cursor of its own. */

use std::alloc::{GlobalAlloc, Layout, System};
use std::any::TypeId;
use std::hint::black_box;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

use solverforge_config::{
    CartesianProductConfig, ChangeMoveConfig, ConstructionHeuristicConfig,
    ConstructionHeuristicType, KOptMoveSelectorConfig, ListChangeMoveConfig, ListSwapMoveConfig,
    LocalSearchConfig, MoveSelectorConfig, NearbyListChangeMoveConfig, NearbyListSwapMoveConfig,
    PhaseConfig, RuinRecreateMoveSelectorConfig, SelectionOrder, SolverConfig,
    SublistChangeMoveConfig, SublistSwapMoveConfig, UnionMoveSelectorConfig, VariableTargetConfig,
};
use solverforge_core::domain::{
    EntityCollectionExtractor, EntityDescriptor, PlanningSolution, SolutionDescriptor,
    VariableDescriptor,
};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use solverforge_solver::__bench::{CandidateVisitor, CompiledNeighborhood, CompiledRuntimeRunner};
use solverforge_solver::{
    CandidateTraceCoordinate, CandidateTraceIdentity, CrossEntityDistanceMeter, ListVariableSlot,
    Move, MoveCandidateRef, MoveStreamContext, Phase, RuntimeModel, ScalarVariableSlot,
    SearchContext, SolverScope, ValueSource, VariableSlot,
};

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

include!("selector_cursor_gate/model.rs");

fn mix(hash: &mut u64, value: usize) {
    *hash ^= value as u64;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
}

fn mix_u64(hash: &mut u64, value: u64) {
    *hash ^= value;
    *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
}

fn iterations() -> usize {
    *ITERATIONS.get().expect("benchmark iterations must be set")
}

fn emit(case: &str, expected_count: usize, run: impl FnOnce() -> (usize, u64, u64)) {
    reset_allocator_metrics();
    let started = Instant::now();
    // (count, cross-tree semantic order hash, per-tree identity hash). The
    // identity hash records the tabu-encoding-sensitive stream for determinism
    // only; it is deliberately not compared across trees because an identity
    // encoding migration moves it by design.
    let (count, order_hash, identity_hash) = run();
    let elapsed = started.elapsed();
    assert_eq!(
        count,
        expected_count * iterations(),
        "{case} candidate count changed"
    );
    println!(
        "{{\"case\":\"{case}\",\"iterations\":{},\"candidate_count\":{expected_count},\"total_candidate_count\":{count},\"order_hash\":{order_hash},\"identity_hash\":{identity_hash},\"wall_ns\":{},\"allocations\":{},\"allocated_bytes\":{},\"peak_live_bytes\":{}}}",
        iterations(),
        elapsed.as_nanos(),
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
        PEAK_LIVE_BYTES.load(Ordering::Relaxed).max(0),
    );
}

include!("selector_cursor_gate/neighborhood_driver.rs");
include!("selector_cursor_gate/list_cases.rs");
include!("selector_cursor_gate/scalar_cases.rs");
include!("selector_cursor_gate/construction_cases.rs");
include!("selector_cursor_gate/scoring_cases.rs");

include!("selector_cursor_gate/case_dispatch.rs");

fn main() {
    let runtime_case = std::env::args().nth(1).unwrap_or_else(|| "all".to_string());
    let case = BUILD_CASE.unwrap_or(runtime_case.as_str());
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
    let mut matched = false;
    run_case!(case, matched, "list_change", list_change);
    run_case!(case, matched, "list_swap", list_swap);
    run_case!(case, matched, "nearby_change", nearby_change);
    run_case!(case, matched, "nearby_swap", nearby_swap);
    run_case!(case, matched, "sublist_change", sublist_change);
    run_case!(case, matched, "sublist_swap", sublist_swap);
    run_case!(case, matched, "union", union);
    run_case!(case, matched, "cartesian", cartesian);
    run_case!(case, matched, "k_opt", k_opt);
    run_case!(case, matched, "ruin", ruin);
    run_case!(case, matched, "scalar_change", scalar_change);
    run_case!(case, matched, "construction_full", construction_full);
    run_case!(
        case,
        matched,
        "construction_first_fit",
        construction_first_fit
    );
    run_case!(case, matched, "scoring", scoring);
    run_case!(case, matched, "scoring_weights_one", scoring_weights_one);
    run_case!(
        case,
        matched,
        "scoring_weights_mixed",
        scoring_weights_mixed
    );
    assert!(
        matched || case == "all",
        "unknown selector benchmark case `{case}`"
    );
}
