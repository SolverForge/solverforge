/* Per-thread heap allocation counter for the crate's test binary.

Each thread counts its own allocations, so a test that runs its work on one
thread reads an exact count no matter what concurrently running tests do on
theirs.
*/

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;

thread_local! {
    static THREAD_ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        THREAD_ALLOCATIONS.with(|count| count.set(count.get() + 1));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        THREAD_ALLOCATIONS.with(|count| count.set(count.get() + 1));
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

/// Heap allocations made so far on the calling thread.
pub(crate) fn thread_allocations() -> usize {
    THREAD_ALLOCATIONS.with(Cell::get)
}
