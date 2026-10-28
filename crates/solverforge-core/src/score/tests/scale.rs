// Exact integer scaling for every built-in score type.

use crate::score::*;

const BEYOND_F64: i64 = (1 << 53) + 1;

#[test]
fn soft_scale_is_exact_beyond_f64_precision() {
    let score = SoftScore::of(BEYOND_F64);
    assert_eq!(score.scale(1), score);
    assert_eq!(score.scale(0), SoftScore::ZERO);
    assert_eq!(score.scale(3), SoftScore::of(BEYOND_F64 * 3));
    assert_eq!(score.scale(-1), -score);
    assert_ne!(
        score.multiply(1.0),
        score,
        "multiply(f64) rounds above 2^53"
    );
}

#[test]
fn hard_soft_scale_scales_every_level() {
    let score = HardSoftScore::of(-2, BEYOND_F64);
    assert_eq!(score.scale(3), HardSoftScore::of(-6, BEYOND_F64 * 3));
    assert_eq!(score.scale(0), HardSoftScore::ZERO);
}

#[test]
fn hard_medium_soft_scale_scales_every_level() {
    let score = HardMediumSoftScore::of(-1, 2, -BEYOND_F64);
    assert_eq!(
        score.scale(3),
        HardMediumSoftScore::of(-3, 6, -BEYOND_F64 * 3)
    );
    assert_eq!(score.scale(1), score);
}

#[test]
fn hard_soft_decimal_scale_keeps_fixed_point_exact() {
    let score = HardSoftDecimalScore::of_scaled(-3_050_001, 7);
    assert_eq!(
        score.scale(3),
        HardSoftDecimalScore::of_scaled(-9_150_003, 21)
    );
    assert_eq!(score.scale(0), HardSoftDecimalScore::ZERO);
}

#[test]
fn bendable_scale_scales_every_level() {
    let score = BendableScore::<2, 1>::of([-1, 4], [BEYOND_F64]);
    assert_eq!(
        score.scale(3),
        BendableScore::<2, 1>::of([-3, 12], [BEYOND_F64 * 3])
    );
    assert_eq!(score.scale(0), BendableScore::<2, 1>::zero());
}

#[test]
#[should_panic(expected = "overflows i64")]
fn scale_panics_on_overflow() {
    let _ = HardSoftScore::of(i64::MAX / 2 + 1, 0).scale(2);
}
