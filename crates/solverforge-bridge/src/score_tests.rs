use solverforge_core::score::{ParseableScore, Score};

use crate::{scoped_dynamic_score_family, DynamicScore, DynamicScoreFamily};

#[test]
fn dynamic_score_keeps_static_three_level_score_contract() {
    let score = DynamicScore::hard_medium_soft(-1, 2, 3);

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
fn dynamic_score_displays_declared_family() {
    assert_eq!(DynamicScore::soft(3).to_string(), "3");
    assert_eq!(DynamicScore::hard_soft(-1, 3).to_string(), "-1hard/3soft");
    assert_eq!(
        DynamicScore::hard_soft_decimal(-100_000, 150_500).to_string(),
        "-1hard/1.505soft"
    );
    assert_eq!(
        DynamicScore::hard_medium_soft(-1, 2, 3).to_string(),
        "-1hard/2medium/3soft"
    );
}

#[test]
fn dynamic_score_zero_uses_scoped_declared_family() {
    let score = scoped_dynamic_score_family(DynamicScoreFamily::HardSoftDecimal, || {
        <DynamicScore as Score>::zero()
    });

    assert_eq!(score.to_string(), "0hard/0soft");
}

#[test]
fn dynamic_score_parses_standard_score_strings() {
    assert_eq!(
        DynamicScore::parse("-1hard/2medium/3soft").unwrap(),
        DynamicScore::hard_medium_soft(-1, 2, 3)
    );
    assert_eq!(
        DynamicScore::parse("-1hard/3soft").unwrap(),
        DynamicScore::hard_soft(-1, 3)
    );
    assert_eq!(
        DynamicScore::parse("-1hard/1.505soft").unwrap(),
        DynamicScore::hard_soft_decimal(-100_000, 150_500)
    );
    assert_eq!(DynamicScore::parse("3").unwrap(), DynamicScore::soft(3));
}
