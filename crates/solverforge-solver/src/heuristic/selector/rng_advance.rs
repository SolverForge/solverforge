//! Atomic continuity carry for selector-owned random streams.
//!
//! Standalone descriptor/heuristic selectors own their random continuity as a
//! monotonically advancing `AtomicU64` carry: each cursor open advances the
//! carry once (splitmix64) and derives its per-open generator from the drawn
//! value. No interior `RefCell`, no leaked allocations, no unsafe Send impls;
//! sequence continuity is preserved across opens.

use std::sync::atomic::{AtomicU64, Ordering};

/* One splitmix64 step: advances the carry and returns a mixed draw. */
#[inline]
pub(crate) fn advance_carry(state: &AtomicU64) -> u64 {
    let next = state
        .load(Ordering::Relaxed)
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    state.store(next, Ordering::Relaxed);
    let mut z = next;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11E5);
    z ^ (z >> 31)
}

/* Initializes a carry from a user seed so distinct seeds give distinct streams. */
#[inline]
pub(crate) fn carry_from_seed(seed: u64) -> u64 {
    let mut z = seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11E5);
    z ^ (z >> 31)
}
