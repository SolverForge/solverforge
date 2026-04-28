use std::fmt;
use std::ops::{Add, Neg, Sub};

use crate::score::{Score, ScoreLevel};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct CustomScore {
    hard: i64,
    soft: i64,
}

impl CustomScore {
    fn of(hard: i64, soft: i64) -> Self {
        Self { hard, soft }
    }
}

impl fmt::Display for CustomScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}hard/{}soft", self.hard, self.soft)
    }
}

impl Add for CustomScore {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::of(self.hard + rhs.hard, self.soft + rhs.soft)
    }
}

impl Sub for CustomScore {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::of(self.hard - rhs.hard, self.soft - rhs.soft)
    }
}

impl Neg for CustomScore {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::of(-self.hard, -self.soft)
    }
}

impl Score for CustomScore {
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    fn zero() -> Self {
        Self::of(0, 0)
    }

    fn levels_count() -> usize {
        2
    }

    fn level_number(&self, index: usize) -> i64 {
        match index {
            0 => self.hard,
            1 => self.soft,
            _ => panic!("CustomScore has 2 levels, got index {}", index),
        }
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(levels.len(), 2);
        Self::of(levels[0], levels[1])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        Self::of(
            (self.hard as f64 * multiplicand).round() as i64,
            (self.soft as f64 * multiplicand).round() as i64,
        )
    }

    fn divide(&self, divisor: f64) -> Self {
        Self::of(
            (self.hard as f64 / divisor).round() as i64,
            (self.soft as f64 / divisor).round() as i64,
        )
    }

    fn abs(&self) -> Self {
        Self::of(self.hard.abs(), self.soft.abs())
    }

    fn to_scalar(&self) -> f64 {
        (self.hard as f64 * 1_000_000.0) + self.soft as f64
    }

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Hard,
            1 => ScoreLevel::Soft,
            _ => panic!("CustomScore has 2 levels, got index {}", index),
        }
    }
}

#[test]
fn custom_score_implements_required_level_number() {
    let score = CustomScore::of(-3, -14);

    assert_eq!(score.level_number(0), -3);
    assert_eq!(score.level_number(1), -14);
    assert_eq!(score.to_level_numbers(), vec![-3, -14]);
}

#[test]
#[should_panic]
fn custom_score_level_number_panics_out_of_range() {
    let _ = CustomScore::of(0, 0).level_number(2);
}
