// Global time-tracking statics and helpers for elapsed-time formatting.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

pub(crate) static EPOCH: OnceLock<Instant> = OnceLock::new();
pub(crate) static SOLVE_START_NANOS: AtomicU64 = AtomicU64::new(0);

// Marks the start of solving for elapsed time tracking.
pub(crate) fn mark_solve_start() {
    let epoch = EPOCH.get_or_init(Instant::now);
    let nanos = epoch.elapsed().as_nanos() as u64;
    SOLVE_START_NANOS.store(nanos, Ordering::Relaxed);
}

// Returns elapsed time in seconds since solve start.
pub(crate) fn elapsed_secs() -> f64 {
    let Some(epoch) = EPOCH.get() else {
        return 0.0;
    };
    let start_nanos = SOLVE_START_NANOS.load(Ordering::Relaxed);
    let now_nanos = epoch.elapsed().as_nanos() as u64;
    (now_nanos - start_nanos) as f64 / 1_000_000_000.0
}
