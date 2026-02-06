//! HardSoftScore - Two-level score with hard and soft constraints

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use super::traits::{ParseableScore, Score, ScoreParseError};

/// A score with separate hard and soft constraint levels.
///
/// Hard constraints must be satisfied for a solution to be feasible.
/// Soft constraints are optimization objectives.
///
/// When comparing scores:
/// 1. Hard scores are compared first
/// 2. Soft scores are only compared when hard scores are equal
///
/// # Examples
///
/// ```
/// use solverforge_core::HardSoftScore;
///
/// let score1 = HardSoftScore::of(-1, -100);  // 1 hard constraint broken
/// let score2 = HardSoftScore::of(0, -200);   // Feasible but poor soft score
///
/// // Feasible solutions are always better than infeasible ones
/// assert!(score2 > score1);
///
/// let score3 = HardSoftScore::of(0, -50);    // Better soft score
/// assert!(score3 > score2);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HardSoftScore {
    hard: i64,
    soft: i64,
}

impl HardSoftScore {
    /// The zero score.
    pub const ZERO: HardSoftScore = HardSoftScore { hard: 0, soft: 0 };

    /// One hard constraint penalty.
    pub const ONE_HARD: HardSoftScore = HardSoftScore { hard: 1, soft: 0 };

    /// One soft constraint penalty.
    pub const ONE_SOFT: HardSoftScore = HardSoftScore { hard: 0, soft: 1 };

    /// Creates a new HardSoftScore.
    #[inline]
    pub const fn of(hard: i64, soft: i64) -> Self {
        HardSoftScore { hard, soft }
    }

    /// Creates a score with only a hard component.
    #[inline]
    pub const fn of_hard(hard: i64) -> Self {
        HardSoftScore { hard, soft: 0 }
    }

    /// Creates a score with only a soft component.
    #[inline]
    pub const fn of_soft(soft: i64) -> Self {
        HardSoftScore { hard: 0, soft }
    }

    /// Returns the hard score component.
    #[inline]
    pub const fn hard(&self) -> i64 {
        self.hard
    }

    /// Returns the soft score component.
    #[inline]
    pub const fn soft(&self) -> i64 {
        self.soft
    }

    /// Returns the hard score as a new HardSoftScore.
    pub const fn hard_score(&self) -> HardSoftScore {
        HardSoftScore::of_hard(self.hard)
    }

    /// Returns the soft score as a new HardSoftScore.
    pub const fn soft_score(&self) -> HardSoftScore {
        HardSoftScore::of_soft(self.soft)
    }
}

impl Score for HardSoftScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    #[inline]
    fn zero() -> Self {
        HardSoftScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        2
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        vec![self.hard, self.soft]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(levels.len(), 2, "HardSoftScore requires exactly 2 levels");
        HardSoftScore::of(levels[0], levels[1])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        let hard = (self.hard as f64 * multiplicand).round() as i64;
        let soft = (self.soft as f64 * multiplicand).round() as i64;
        HardSoftScore::of(hard, soft)
    }

    fn divide(&self, divisor: f64) -> Self {
        let hard = (self.hard as f64 / divisor).round() as i64;
        let soft = (self.soft as f64 / divisor).round() as i64;
        HardSoftScore::of(hard, soft)
    }

    fn abs(&self) -> Self {
        HardSoftScore::of(self.hard.abs(), self.soft.abs())
    }

    #[inline]
    fn to_scalar(&self) -> f64 {
        self.hard as f64 * 1_000_000.0 + self.soft as f64
    }
}

impl Ord for HardSoftScore {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare hard scores first, then soft scores
        match self.hard.cmp(&other.hard) {
            Ordering::Equal => self.soft.cmp(&other.soft),
            other => other,
        }
    }
}

impl PartialOrd for HardSoftScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for HardSoftScore {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        HardSoftScore::of(self.hard + other.hard, self.soft + other.soft)
    }
}

impl Sub for HardSoftScore {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        HardSoftScore::of(self.hard - other.hard, self.soft - other.soft)
    }
}

impl Neg for HardSoftScore {
    type Output = Self;

    fn neg(self) -> Self {
        HardSoftScore::of(-self.hard, -self.soft)
    }
}

impl fmt::Debug for HardSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HardSoftScore({}, {})", self.hard, self.soft)
    }
}

impl fmt::Display for HardSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}hard/{}soft", self.hard, self.soft)
    }
}

impl ParseableScore for HardSoftScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();

        // Format: "0hard/-100soft" or "-1hard/0soft"
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(ScoreParseError {
                message: format!(
                    "Invalid HardSoftScore format '{}': expected 'Xhard/Ysoft'",
                    s
                ),
            });
        }

        let hard_str = parts[0]
            .trim()
            .strip_suffix("hard")
            .ok_or_else(|| ScoreParseError {
                message: format!("Hard score part '{}' must end with 'hard'", parts[0]),
            })?;

        let soft_str = parts[1]
            .trim()
            .strip_suffix("soft")
            .ok_or_else(|| ScoreParseError {
                message: format!("Soft score part '{}' must end with 'soft'", parts[1]),
            })?;

        let hard = hard_str.parse::<i64>().map_err(|e| ScoreParseError {
            message: format!("Invalid hard score '{}': {}", hard_str, e),
        })?;

        let soft = soft_str.parse::<i64>().map_err(|e| ScoreParseError {
            message: format!("Invalid soft score '{}': {}", soft_str, e),
        })?;

        Ok(HardSoftScore::of(hard, soft))
    }

    fn to_string_repr(&self) -> String {
        format!("{}hard/{}soft", self.hard, self.soft)
    }
}
