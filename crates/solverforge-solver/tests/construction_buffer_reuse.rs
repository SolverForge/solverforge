//! A construction placement cursor reuses the buffers of a placement handed
//! back to it, so after the first placement the next ones allocate no
//! candidate store or value buffer.
//!
//! This is its own test binary because it counts the process's allocations.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, PhaseConfig, SolverConfig,
};
use solverforge_solver::__bench::CompiledRuntimeRunner;

mod support;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

// SAFETY: every call forwards to the system allocator unchanged.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn a_recycled_placement_lends_its_buffers_to_the_next() {
    let config = SolverConfig {
        random_seed: Some(7),
        phases: vec![PhaseConfig::ConstructionHeuristic(
            ConstructionHeuristicConfig {
                construction_heuristic_type: ConstructionHeuristicType::FirstFit,
                ..ConstructionHeuristicConfig::default()
            },
        )],
        ..SolverConfig::default()
    };
    let placer = CompiledRuntimeRunner::compile(&config, support::context())
        .expect("the runner compiles")
        .into_scalar_construction_placer()
        .expect("the first phase is a scalar construction");
    let director = support::director(support::unassigned(4));
    let mut placements = placer.open_cursor(&director);

    let mut placement_allocations = Vec::with_capacity(8);
    let mut values = Vec::with_capacity(8);
    loop {
        let before = ALLOCATIONS.load(Ordering::Relaxed);
        let Some(mut placement) = placements.next_placement(&director) else {
            break;
        };
        values.clear();
        while let Some(id) = placement.next_candidate(director.working_solution()) {
            values.push(placement.candidate_assignment(id).expect("live").1);
            assert!(placement.release_candidate(id));
        }
        placement_allocations.push(ALLOCATIONS.load(Ordering::Relaxed) - before);
        assert_eq!(values, [0, 1, 2, 3]);
        placements.recycle(placement);
    }

    // Every task is a placement, offered every worker in order. The first
    // placement allocates the slot's entity order and the buffers; each later
    // one allocates only its construction target, reusing the recycled
    // candidate store and value buffer.
    assert_eq!(placement_allocations.len(), 4);
    assert!(placement_allocations[0] > 1, "{placement_allocations:?}");
    assert_eq!(placement_allocations[1..], [1, 1, 1]);
}
