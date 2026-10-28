// Exact integer level scaling shared by every built-in score type.

/* Multiplies one score level by an integer factor.

# Panics
Panics when the product overflows `i64`. Scaling is strict in every build
profile so a constraint weight multiplier can never wrap a score silently.
*/
#[inline(always)]
pub(crate) fn scale_level(level: i64, factor: i64) -> i64 {
    match level.checked_mul(factor) {
        Some(scaled) => scaled,
        None => scale_overflow(factor),
    }
}

/* Panics for a score level that overflowed `i64` while scaled by `factor`.
Kept out of line so scaling inlines into every weighted delta as multiplies
and one branch. */
#[cold]
#[inline(never)]
pub(crate) fn scale_overflow(factor: i64) -> ! {
    panic!("a score level overflows i64 when scaled by {factor}")
}
