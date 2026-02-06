//! SimpleScore - Single-level score implementation

use std::cmp::Ordering;
use std::fmt;

use super::traits::{ParseableScore, Score, ScoreParseError};
use super::ScoreLevel;

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    impl_score_scale!(SimpleScore { score } => of);

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Soft,
            _ => panic!("SimpleScore has 1 level, got index {}", index),
        }
    }
}

impl Ord for SimpleScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl_score_ops!(SimpleScore { score } => of);

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

// SimpleScore has custom parse logic (optional "init" suffix) so no macro.
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
