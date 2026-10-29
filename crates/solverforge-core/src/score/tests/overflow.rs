// Score arithmetic panics on overflow in every build profile; run these
// with `cargo test --release` too, where plain `i64` operators would wrap.

use crate::score::*;

#[test]
#[should_panic(expected = "score addition overflows i64")]
fn soft_score_addition_overflow_panics() {
    let _ = SoftScore::of(std::hint::black_box(i64::MAX)) + SoftScore::of(1);
}

#[test]
#[should_panic(expected = "score subtraction overflows i64")]
fn soft_score_subtraction_overflow_panics() {
    let _ = SoftScore::of(std::hint::black_box(i64::MIN)) - SoftScore::of(1);
}

#[test]
#[should_panic(expected = "score addition overflows i64")]
fn hard_soft_score_addition_overflow_panics() {
    let _ = HardSoftScore::of_soft(std::hint::black_box(i64::MAX)) + HardSoftScore::ONE_SOFT;
}

#[test]
#[should_panic(expected = "score negation overflows i64")]
fn hard_soft_score_negation_overflow_panics() {
    let _ = -HardSoftScore::of_hard(std::hint::black_box(i64::MIN));
}

#[test]
#[should_panic(expected = "score subtraction overflows i64")]
fn hard_soft_score_subtraction_overflow_panics() {
    let _ = HardSoftScore::of_hard(std::hint::black_box(i64::MIN)) - HardSoftScore::ONE_HARD;
}

#[test]
#[should_panic(expected = "score addition overflows i64")]
fn hard_medium_soft_score_addition_overflow_panics() {
    let _ = HardMediumSoftScore::of_medium(std::hint::black_box(i64::MAX))
        + HardMediumSoftScore::ONE_MEDIUM;
}

#[test]
#[should_panic(expected = "score negation overflows i64")]
fn hard_medium_soft_score_negation_overflow_panics() {
    let _ = -HardMediumSoftScore::of_soft(std::hint::black_box(i64::MIN));
}

#[test]
#[should_panic(expected = "score addition overflows i64")]
fn decimal_score_addition_overflow_panics() {
    let _ = HardSoftDecimalScore::of_soft_scaled(std::hint::black_box(i64::MAX))
        + HardSoftDecimalScore::of_soft_scaled(1);
}

#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn decimal_score_unscaled_constructor_overflow_panics() {
    let _ = HardSoftDecimalScore::of_hard(std::hint::black_box(-100_000_000_000_000));
}

#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn decimal_score_of_overflow_panics() {
    let _ = HardSoftDecimalScore::of(0, std::hint::black_box(92_233_720_368_548));
}

#[test]
#[should_panic(expected = "score addition overflows i64")]
fn bendable_score_addition_overflow_panics() {
    let _ = BendableScore::<1, 2>::of([0], [0, std::hint::black_box(i64::MAX)])
        + BendableScore::one_soft(1);
}

#[test]
#[should_panic(expected = "score negation overflows i64")]
fn bendable_score_negation_overflow_panics() {
    let _ = -BendableScore::<2, 1>::of([0, std::hint::black_box(i64::MIN)], [0]);
}

#[test]
#[should_panic(expected = "score subtraction overflows i64")]
fn bendable_score_subtraction_overflow_panics() {
    let _ = BendableScore::<1, 1>::of([std::hint::black_box(i64::MIN)], [0])
        - BendableScore::one_hard(0);
}

#[test]
#[should_panic(expected = "score absolute value overflows i64")]
fn abs_of_min_level_panics() {
    let _ = HardSoftScore::of_soft(std::hint::black_box(i64::MIN)).abs();
}

#[test]
#[should_panic(expected = "a score cannot be multiplied or divided by NaN")]
fn multiply_by_nan_panics() {
    let _ = HardSoftScore::of_soft(5).multiply(std::hint::black_box(f64::NAN));
}

#[test]
#[should_panic(expected = "score multiplication overflows i64")]
fn multiply_out_of_range_panics() {
    let _ = HardSoftScore::of_soft(5).multiply(std::hint::black_box(1e300));
}

#[test]
#[should_panic(expected = "a score cannot be multiplied or divided by 0")]
fn divide_by_zero_panics() {
    let _ = HardSoftScore::of_soft(5).divide(std::hint::black_box(0.0));
}

#[test]
#[should_panic(expected = "a score cannot be multiplied or divided by NaN")]
fn bendable_multiply_by_nan_panics() {
    let _ = BendableScore::<1, 1>::of([1], [1]).multiply(std::hint::black_box(f64::NAN));
}

#[test]
fn in_range_arithmetic_is_unchanged() {
    let a = HardSoftScore::of(-3, i64::MAX - 1);
    let d = HardSoftScore::ONE_SOFT;
    assert_eq!(a + d - d, a);
    assert_eq!(
        -HardSoftScore::of_hard(i64::MIN + 1),
        HardSoftScore::of_hard(i64::MAX)
    );
    assert_eq!(HardSoftScore::of_soft(-7).abs(), HardSoftScore::of_soft(7));
    assert_eq!(
        HardSoftScore::of_soft(5).multiply(2.5),
        HardSoftScore::of_soft(13)
    );
    assert_eq!(
        HardSoftScore::of_soft(5).divide(2.0),
        HardSoftScore::of_soft(3)
    );
    assert_eq!(
        BendableScore::<1, 2>::of([1], [2, 3]) - BendableScore::of([1], [1, 1]),
        BendableScore::of([0], [1, 2])
    );
    assert_eq!(HardSoftDecimalScore::of(-2, 1).hard_scaled(), -200_000);
}
