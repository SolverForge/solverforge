// Global time-tracking statics and helpers for elapsed-time formatting.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

pub(crate) static SOLVE_START: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();

// Marks the start of solving for elapsed time tracking.
pub(crate) fn mark_solve_start() {
    let start = SOLVE_START.get_or_init(|| Mutex::new(None));
    *start.lock().unwrap() = Some(Instant::now());
}

// Returns exact elapsed time since solve start.
pub(crate) fn elapsed() -> Duration {
    let Some(start) = SOLVE_START.get() else {
        return Duration::ZERO;
    };

    start
        .lock()
        .unwrap()
        .as_ref()
        .map(Instant::elapsed)
        .unwrap_or(Duration::ZERO)
}
