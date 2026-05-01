use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use solverforge_core::score::{BendableScore, HardSoftScore, Score, ScoreLevel, SoftScore};

use super::hard_delta::{hard_score_delta, HardScoreDelta};

#[test]
fn single_hard_level_behavior_is_unchanged() {
    assert_eq!(
        hard_score_delta(HardSoftScore::of(-1, 0), HardSoftScore::of(0, -100)),
        Some(HardScoreDelta::Improving)
    );
    assert_eq!(
        hard_score_delta(HardSoftScore::of(0, 0), HardSoftScore::of(-1, 100)),
        Some(HardScoreDelta::Worse)
    );
    assert_eq!(
        hard_score_delta(HardSoftScore::of(0, 0), HardSoftScore::of(0, 100)),
        Some(HardScoreDelta::Neutral)
    );
}

#[test]
fn second_bendable_hard_level_can_improve_required_repairs() {
    assert_eq!(
        hard_score_delta(
            BendableScore::<2, 1>::of([0, -10], [0]),
            BendableScore::<2, 1>::of([0, -5], [-100])
        ),
        Some(HardScoreDelta::Improving)
    );
}

#[test]
fn second_bendable_hard_level_can_regress() {
    assert_eq!(
        hard_score_delta(
            BendableScore::<2, 1>::of([0, -5], [0]),
            BendableScore::<2, 1>::of([0, -10], [100])
        ),
        Some(HardScoreDelta::Worse)
    );
}

#[test]
fn earlier_hard_level_dominates_later_hard_levels() {
    assert_eq!(
        hard_score_delta(
            BendableScore::<2, 1>::of([-1, 0], [0]),
            BendableScore::<2, 1>::of([-2, 100], [100])
        ),
        Some(HardScoreDelta::Worse)
    );
    assert_eq!(
        hard_score_delta(
            BendableScore::<2, 1>::of([-1, 100], [0]),
            BendableScore::<2, 1>::of([0, -100], [-100])
        ),
        Some(HardScoreDelta::Improving)
    );
}

#[test]
fn soft_level_changes_do_not_affect_hard_delta() {
    assert_eq!(
        hard_score_delta(
            BendableScore::<2, 1>::of([0, -5], [0]),
            BendableScore::<2, 1>::of([0, -5], [100])
        ),
        Some(HardScoreDelta::Neutral)
    );
}

#[test]
fn score_without_hard_levels_has_no_hard_delta() {
    assert_eq!(hard_score_delta(SoftScore::of(0), SoftScore::of(100)), None);
}

#[test]
fn custom_non_contiguous_hard_labels_are_compared_in_level_order() {
    assert_eq!(
        hard_score_delta(
            SplitHardScore::new([0, 1_000, -10]),
            SplitHardScore::new([0, -1_000, -5])
        ),
        Some(HardScoreDelta::Improving)
    );
    assert_eq!(
        hard_score_delta(
            SplitHardScore::new([0, 1_000, -5]),
            SplitHardScore::new([0, -1_000, -10])
        ),
        Some(HardScoreDelta::Worse)
    );
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SplitHardScore {
    levels: [i64; 3],
}

impl SplitHardScore {
    const fn new(levels: [i64; 3]) -> Self {
        Self { levels }
    }
}

impl Ord for SplitHardScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.levels.cmp(&other.levels)
    }
}

impl PartialOrd for SplitHardScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for SplitHardScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new([
            self.levels[0] + rhs.levels[0],
            self.levels[1] + rhs.levels[1],
            self.levels[2] + rhs.levels[2],
        ])
    }
}

impl Sub for SplitHardScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new([
            self.levels[0] - rhs.levels[0],
            self.levels[1] - rhs.levels[1],
            self.levels[2] - rhs.levels[2],
        ])
    }
}

impl Neg for SplitHardScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new([-self.levels[0], -self.levels[1], -self.levels[2]])
    }
}

impl fmt::Display for SplitHardScore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}hard/{}soft/{}hard",
            self.levels[0], self.levels[1], self.levels[2]
        )
    }
}

impl Score for SplitHardScore {
    fn is_feasible(&self) -> bool {
        self.levels[0] >= 0 && self.levels[2] >= 0
    }

    fn zero() -> Self {
        Self::default()
    }

    fn levels_count() -> usize {
        3
    }

    fn level_number(&self, index: usize) -> i64 {
        self.levels[index]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        Self::new([levels[0], levels[1], levels[2]])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        Self::new([
            (self.levels[0] as f64 * multiplicand).round() as i64,
            (self.levels[1] as f64 * multiplicand).round() as i64,
            (self.levels[2] as f64 * multiplicand).round() as i64,
        ])
    }

    fn divide(&self, divisor: f64) -> Self {
        Self::new([
            (self.levels[0] as f64 / divisor).round() as i64,
            (self.levels[1] as f64 / divisor).round() as i64,
            (self.levels[2] as f64 / divisor).round() as i64,
        ])
    }

    fn abs(&self) -> Self {
        Self::new([
            self.levels[0].abs(),
            self.levels[1].abs(),
            self.levels[2].abs(),
        ])
    }

    fn to_scalar(&self) -> f64 {
        (self.levels[0] as f64 * 1_000_000_000_000.0)
            + (self.levels[1] as f64 * 1_000_000.0)
            + self.levels[2] as f64
    }

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 | 2 => ScoreLevel::Hard,
            1 => ScoreLevel::Soft,
            _ => panic!("SplitHardScore has 3 levels, got index {}", index),
        }
    }
}
