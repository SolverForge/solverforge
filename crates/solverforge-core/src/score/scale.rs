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

/* Panics for a score addition, subtraction, negation or absolute value whose
result does not fit `i64`. Score arithmetic computes every level with an
overflowing operation and branches here once; it never wraps, because a
wrapped level flips sign and turns a penalty into a reward. Saturating instead
would break `a + d - d == a`, which incremental scoring relies on. */
/* Level arithmetic for the overflowing score operations. Each wraps and ORs
whether the level overflowed into `overflow`, so a multi-level score is
checked once by its caller. */
#[inline(always)]
pub(crate) fn add_level(left: i64, right: i64, overflow: &mut bool) -> i64 {
    track_overflow(left.overflowing_add(right), overflow)
}

#[inline(always)]
pub(crate) fn sub_level(left: i64, right: i64, overflow: &mut bool) -> i64 {
    track_overflow(left.overflowing_sub(right), overflow)
}

#[inline(always)]
pub(crate) fn neg_level(level: i64, overflow: &mut bool) -> i64 {
    track_overflow(level.overflowing_neg(), overflow)
}

/* Returns one level of an overflowing operation and records whether it
wrapped, so score arithmetic checks all levels with a single branch. */
#[inline(always)]
pub(crate) fn track_overflow((level, wrapped): (i64, bool), overflow: &mut bool) -> i64 {
    *overflow |= wrapped;
    level
}

#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn arithmetic_overflow(operation: &'static str) -> ! {
    panic!("score {operation} overflows i64")
}

/* Rounds `level * factor` to a level, for `multiply` and `divide`, recording
in `overflow` when the rounded result is outside `i64` (an `as` cast would
silently saturate). Every level converts unconditionally and the caller checks
once with `float_overflow`. */
#[inline(always)]
pub(crate) fn float_level(level: i64, factor: f64, overflow: &mut bool) -> i64 {
    // 2^63 is exactly representable; every f64 in [-2^63, 2^63) fits i64.
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    let rounded = (level as f64 * factor).round();
    // A NaN product (from a non-finite factor) also fails this test.
    // Non-short-circuit `&` keeps the test a flag instead of a branch.
    #[allow(clippy::manual_range_contains)]
    let in_range = (rounded >= -LIMIT) & (rounded < LIMIT);
    *overflow |= !in_range;
    rounded as i64
}

/* Panics for a `multiply` by a non-finite factor, a `divide` by zero or a
non-finite divisor, or a result that does not fit `i64`. `argument` is the
caller's multiplicand or divisor. */
#[cold]
#[inline(never)]
#[track_caller]
pub(crate) fn float_overflow(argument: f64) -> ! {
    if !argument.is_finite() || argument == 0.0 {
        non_finite_factor(argument);
    }
    arithmetic_overflow("multiplication")
}

#[cold]
#[inline(never)]
#[track_caller]
fn non_finite_factor(factor: f64) -> ! {
    panic!("a score cannot be multiplied or divided by {factor}")
}
