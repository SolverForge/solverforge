//! A join whose rows leave and re-enter on every move reuses the output-link
//! buckets it emptied, so steady-state incremental scoring allocates nothing.
//!
//! This is its own test binary because it replaces the global allocator. It
//! counts only the test thread's allocations, so the harness's other threads
//! cannot touch the count.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use solverforge_core::score::SoftScore;
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::stream::joiner::equal_bi;
use solverforge_scoring::stream::{source, ChangeSource, ConstraintFactory};

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count_allocation() {
    // `try_with` skips allocations made while the thread-local is torn down.
    let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
}

// SAFETY: every call forwards to the system allocator unchanged.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Debug)]
struct Item {
    key: u32,
}

struct Model {
    left: Vec<Item>,
    right: Vec<Item>,
}

fn left(model: &Model) -> &[Item] {
    &model.left
}

fn right(model: &Model) -> &[Item] {
    &model.right
}

#[test]
fn rejoining_rows_reuse_their_link_buckets() {
    let mut constraint = ConstraintFactory::<Model, SoftScore>::new()
        .for_each(source(
            left as fn(&Model) -> &[Item],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(right as fn(&Model) -> &[Item], ChangeSource::Descriptor(1)),
            equal_bi(|l: &Item| l.key, |r: &Item| r.key),
        ))
        .penalize(SoftScore::of(1))
        .named("matching keys");
    let mut model = Model {
        left: vec![Item { key: 1 }, Item { key: 2 }],
        right: vec![Item { key: 1 }, Item { key: 1 }, Item { key: 2 }],
    };
    let mut score = constraint.initialize(&model);
    let mut cycle = |model: &mut Model, score: &mut SoftScore| {
        *score = *score + constraint.on_retract(model, 0, 0);
        model.left[0].key = if model.left[0].key == 1 { 2 } else { 1 };
        *score = *score + constraint.on_insert(model, 0, 0);
    };
    // Warm-up: the first moves size every bucket and scratch buffer.
    for _ in 0..4 {
        cycle(&mut model, &mut score);
    }

    let before = ALLOCATIONS.with(Cell::get);
    for _ in 0..64 {
        cycle(&mut model, &mut score);
    }
    let allocated = ALLOCATIONS.with(Cell::get) - before;

    assert_eq!(allocated, 0, "steady-state moves allocate nothing");
    assert_eq!(score, SoftScore::of(-3), "left 0 is back on key 1");
}
