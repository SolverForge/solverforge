// Exact integer level scaling shared by every built-in score type.

/* Multiplies one score level by an integer factor.

# Panics
Panics when the product overflows `i64`. Scaling is strict in every build
profile so a constraint weight multiplier can never wrap a score silently.
*/
#[inline]
pub(crate) fn scale_level(level: i64, factor: i64) -> i64 {
    match level.checked_mul(factor) {
        Some(scaled) => scaled,
        None => panic!("score level {level} overflows i64 when scaled by {factor}"),
    }
}
