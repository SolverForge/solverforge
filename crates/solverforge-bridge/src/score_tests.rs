use solverforge_core::score::{ParseableScore, Score};

use crate::{DynamicScore, DynamicScoreFamily};

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
fn dynamic_score_parses_standard_score_strings() {
    assert_eq!(
        DynamicScore::parse("-1hard/2medium/3soft").unwrap(),
        DynamicScore::hard_medium_soft(-1, 2, 3)
    );
    assert_eq!(
        DynamicScore::parse("-1hard/3soft").unwrap(),
        DynamicScore::hard_soft(-1, 3)
    );
    assert_eq!(DynamicScore::parse("3").unwrap(), DynamicScore::soft(3));
}
