//! SimpleScore - Single-level score implementation

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use super::traits::{ParseableScore, Score, ScoreParseError};

/// A simple score with a single integer value.
///
/// This is the simplest score type, useful when there's only one
/// type of constraint to optimize.
///
/// # Examples
///
/// ```
/// use solverforge_core::{SimpleScore, Score};
///
/// let score1 = SimpleScore::of(-5);
/// let score2 = SimpleScore::of(-3);
///
/// assert!(score2 > score1);  // -3 is better than -5
/// assert!(!score1.is_feasible());  // Negative scores are not feasible
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SimpleScore {
    score: i64,
}

impl SimpleScore {
    /// The zero score.
    pub const ZERO: SimpleScore = SimpleScore { score: 0 };

    /// A score of 1 (useful for incrementing).
    pub const ONE: SimpleScore = SimpleScore { score: 1 };

    /// Creates a new SimpleScore with the given value.
    #[inline]
    pub const fn of(score: i64) -> Self {
        SimpleScore { score }
    }

    /// Returns the score value.
    #[inline]
    pub const fn score(&self) -> i64 {
        self.score
    }
}

impl Score for SimpleScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.score >= 0
    }

    #[inline]
    fn zero() -> Self {
        SimpleScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        1
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        vec![self.score]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(levels.len(), 1, "SimpleScore requires exactly 1 level");
        SimpleScore::of(levels[0])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        SimpleScore::of((self.score as f64 * multiplicand).round() as i64)
    }

    fn divide(&self, divisor: f64) -> Self {
        SimpleScore::of((self.score as f64 / divisor).round() as i64)
    }

    fn abs(&self) -> Self {
        SimpleScore::of(self.score.abs())
    }
}

impl Ord for SimpleScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl PartialOrd for SimpleScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for SimpleScore {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        SimpleScore::of(self.score + other.score)
    }
}

impl Sub for SimpleScore {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        SimpleScore::of(self.score - other.score)
    }
}

impl Neg for SimpleScore {
    type Output = Self;

    fn neg(self) -> Self {
        SimpleScore::of(-self.score)
    }
}

impl fmt::Debug for SimpleScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimpleScore({})", self.score)
    }
}

impl fmt::Display for SimpleScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.score)
    }
}

impl ParseableScore for SimpleScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();
        // Remove optional "init" suffix
        let s = s.strip_suffix("init").unwrap_or(s);

        s.parse::<i64>()
            .map(SimpleScore::of)
            .map_err(|e| ScoreParseError {
                message: format!("Invalid SimpleScore '{}': {}", s, e),
            })
    }

    fn to_string_repr(&self) -> String {
        self.score.to_string()
    }
}

impl From<i64> for SimpleScore {
    fn from(score: i64) -> Self {
        SimpleScore::of(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let score = SimpleScore::of(-5);
        assert_eq!(score.score(), -5);
    }

    #[test]
    fn test_feasibility() {
        assert!(SimpleScore::of(0).is_feasible());
        assert!(SimpleScore::of(10).is_feasible());
        assert!(!SimpleScore::of(-1).is_feasible());
    }

    #[test]
    fn test_comparison() {
        let s1 = SimpleScore::of(-10);
        let s2 = SimpleScore::of(-5);
        let s3 = SimpleScore::of(0);

        assert!(s3 > s2);
        assert!(s2 > s1);
        assert!(s1 < s2);
    }

    #[test]
    fn test_arithmetic() {
        let s1 = SimpleScore::of(10);
        let s2 = SimpleScore::of(3);

        assert_eq!(s1 + s2, SimpleScore::of(13));
        assert_eq!(s1 - s2, SimpleScore::of(7));
        assert_eq!(-s1, SimpleScore::of(-10));
    }

    #[test]
    fn test_multiply_divide() {
        let score = SimpleScore::of(10);

        assert_eq!(score.multiply(2.0), SimpleScore::of(20));
        assert_eq!(score.divide(2.0), SimpleScore::of(5));
    }

    #[test]
    fn test_parse() {
        assert_eq!(SimpleScore::parse("42").unwrap(), SimpleScore::of(42));
        assert_eq!(SimpleScore::parse("-10").unwrap(), SimpleScore::of(-10));
        assert_eq!(SimpleScore::parse("0init").unwrap(), SimpleScore::of(0));
    }

    #[test]
    fn test_level_numbers() {
        let score = SimpleScore::of(-5);
        assert_eq!(score.to_level_numbers(), vec![-5]);
        assert_eq!(SimpleScore::from_level_numbers(&[-5]), score);
    }
}
