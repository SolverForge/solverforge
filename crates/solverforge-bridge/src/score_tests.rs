use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use solverforge_core::score::{HardSoftDecimalScore, ParseableScore, Score};

use crate::{
    active_dynamic_score_family, scoped_dynamic_score_family, DynamicScore, DynamicScoreFamily,
};

fn hash_of(score: DynamicScore) -> u64 {
    let mut hasher = DefaultHasher::new();
    score.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn dynamic_score_keeps_static_three_level_score_contract() {
    let score = DynamicScore::of(-1, 2, 3);

    assert_eq!(DynamicScore::levels_count(), 3);
    assert!(!score.is_feasible());
    assert_eq!(score.to_level_numbers(), vec![-1, 2, 3]);
    assert_eq!(score.family_levels(DynamicScoreFamily::Soft), vec![3]);
    assert_eq!(
        score.family_levels(DynamicScoreFamily::HardSoft),
        vec![-1, 3]
    );
}

#[test]
fn dynamic_score_displays_the_active_family() {
    let display =
        |family, score: DynamicScore| scoped_dynamic_score_family(family, || score.to_string());

    assert_eq!(
        display(DynamicScoreFamily::Soft, DynamicScore::soft(3)),
        "3"
    );
    assert_eq!(
        display(DynamicScoreFamily::HardSoft, DynamicScore::hard_soft(-1, 3)),
        "-1hard/3soft"
    );
    assert_eq!(
        display(
            DynamicScoreFamily::HardSoftDecimal,
            DynamicScore::hard_soft(-100_000, 150_500)
        ),
        "-1hard/1.505soft"
    );
    assert_eq!(
        display(
            DynamicScoreFamily::HardMediumSoft,
            DynamicScore::of(-1, 2, 3)
        ),
        "-1hard/2medium/3soft"
    );
}

#[test]
fn dynamic_score_family_scope_restores_the_previous_family() {
    scoped_dynamic_score_family(DynamicScoreFamily::Soft, || {
        scoped_dynamic_score_family(DynamicScoreFamily::HardSoft, || {
            assert_eq!(active_dynamic_score_family(), DynamicScoreFamily::HardSoft);
        });
        assert_eq!(active_dynamic_score_family(), DynamicScoreFamily::Soft);
    });
    assert_eq!(
        active_dynamic_score_family(),
        DynamicScoreFamily::HardMediumSoft
    );
}

#[test]
fn dynamic_score_parses_standard_score_strings() {
    assert_eq!(
        DynamicScore::parse("-1hard/2medium/3soft").unwrap(),
        DynamicScore::of(-1, 2, 3)
    );
    assert_eq!(
        DynamicScore::parse("-1hard/3soft").unwrap(),
        DynamicScore::hard_soft(-1, 3)
    );
    assert_eq!(
        DynamicScore::parse("-1hard/1.505soft").unwrap(),
        DynamicScore::hard_soft(-100_000, 150_500)
    );
    assert_eq!(DynamicScore::parse("3").unwrap(), DynamicScore::soft(3));
}

#[test]
fn dynamic_score_equality_agrees_with_ordering() {
    let declared = DynamicScore::hard_soft(0, 0);

    assert_eq!(declared.cmp(&DynamicScore::ZERO), Ordering::Equal);
    assert_eq!(declared, DynamicScore::ZERO);
    assert_eq!(hash_of(declared), hash_of(DynamicScore::ZERO));
    assert!(DynamicScore::of(0, 0, 0) > DynamicScore::of(-1, 100, 100));
}

#[test]
fn dynamic_score_zero_does_not_depend_on_the_calling_thread() {
    // Evaluator replicas score on worker threads that never entered the
    // solve's family scope; their zero must equal the solve thread's.
    let solve_thread_zero = scoped_dynamic_score_family(DynamicScoreFamily::HardSoft, || {
        <DynamicScore as Score>::zero()
    });

    assert_eq!(solve_thread_zero, <DynamicScore as Score>::zero());
}

#[test]
fn dynamic_score_parse_inverts_display_for_decimal_models() {
    scoped_dynamic_score_family(DynamicScoreFamily::HardSoftDecimal, || {
        let limit = DynamicScore::parse("0hard/-5soft").unwrap();

        assert_eq!((limit.hard, limit.soft), (0, -500_000));
        assert_eq!(limit.to_string(), "0hard/-5soft");
    });
}

#[test]
fn dynamic_score_scale_is_exact_on_every_level() {
    let beyond_f64 = (1_i64 << 53) + 1;
    let score = DynamicScore::of(-2, 5, beyond_f64);

    assert_eq!(score.scale(3), DynamicScore::of(-6, 15, beyond_f64 * 3));
    assert_eq!(score.scale(1), score);
    assert_eq!(score.scale(0), DynamicScore::ZERO);
}

#[test]
#[should_panic(expected = "overflows i64")]
fn dynamic_score_scale_panics_on_overflow() {
    let _ = DynamicScore::of(0, 0, i64::MAX).scale(2);
}

#[test]
fn dynamic_decimal_scores_print_and_parse_exactly() {
    scoped_dynamic_score_family(DynamicScoreFamily::HardSoftDecimal, || {
        for scaled in [1, -1, -50_000, 9_410_874_450_160_629, i64::MAX, i64::MIN] {
            let score = DynamicScore::hard_soft(scaled, -scaled.saturating_add(1));
            let text = score.to_string();
            assert_eq!(
                text,
                HardSoftDecimalScore::of_scaled(score.hard, score.soft).to_string()
            );
            assert_eq!(DynamicScore::parse(&text).unwrap(), score, "{text}");
        }
    });
}

#[test]
fn dynamic_score_rejects_unrepresentable_decimal_text() {
    scoped_dynamic_score_family(DynamicScoreFamily::HardSoftDecimal, || {
        for input in [
            "NaNhard/0soft",
            "0hard/inf.0soft",
            "1e3.0hard/0soft",
            "0hard/0.000004soft",
            "92233720368548hard/0soft",
            "0hard/92233720368548soft",
        ] {
            assert!(
                DynamicScore::parse(input).is_err(),
                "{input} must not parse"
            );
        }
    });
}

#[test]
#[should_panic(expected = "score addition overflows i64")]
fn dynamic_score_addition_overflow_panics() {
    let _ = DynamicScore::soft(std::hint::black_box(i64::MAX)) + DynamicScore::soft(1);
}

#[test]
#[should_panic(expected = "score negation overflows i64")]
fn dynamic_score_negation_overflow_panics() {
    let _ = -DynamicScore::of(std::hint::black_box(i64::MIN), 0, 0);
}

#[test]
#[should_panic(expected = "a score cannot be divided by 0")]
fn dynamic_score_divide_by_zero_panics() {
    let _ = DynamicScore::soft(5).divide(std::hint::black_box(0.0));
}

#[test]
#[should_panic(expected = "a score cannot be multiplied by NaN")]
fn dynamic_score_multiply_by_nan_panics() {
    let _ = DynamicScore::soft(5).multiply(std::hint::black_box(f64::NAN));
}

#[test]
fn dynamic_score_in_range_arithmetic_is_unchanged() {
    let a = DynamicScore::of(-1, 2, i64::MAX - 1);
    let d = DynamicScore::soft(1);
    assert_eq!(a + d - d, a);
    assert_eq!(DynamicScore::soft(5).divide(2.0), DynamicScore::soft(3));
}
