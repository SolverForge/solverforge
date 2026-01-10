//! HardMediumSoftScore - Three-level score with hard, medium, and soft constraints

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use super::traits::{ParseableScore, Score, ScoreParseError};

/// A score with hard, medium, and soft constraint levels.
///
/// Hard constraints must be satisfied for feasibility.
/// Medium constraints have higher priority than soft constraints.
/// Soft constraints are the lowest priority optimization objectives.
///
/// Comparison order: hard > medium > soft
///
/// # Examples
///
/// ```
/// use solverforge_core::HardMediumSoftScore;
///
/// let score1 = HardMediumSoftScore::of(0, -10, -100);
/// let score2 = HardMediumSoftScore::of(0, -5, -200);
///
/// // Better medium score wins even with worse soft score
/// assert!(score2 > score1);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HardMediumSoftScore {
    hard: i64,
    medium: i64,
    soft: i64,
}

impl HardMediumSoftScore {
    /// The zero score.
    pub const ZERO: HardMediumSoftScore = HardMediumSoftScore {
        hard: 0,
        medium: 0,
        soft: 0,
    };

    /// One hard constraint penalty.
    pub const ONE_HARD: HardMediumSoftScore = HardMediumSoftScore {
        hard: 1,
        medium: 0,
        soft: 0,
    };

    /// One medium constraint penalty.
    pub const ONE_MEDIUM: HardMediumSoftScore = HardMediumSoftScore {
        hard: 0,
        medium: 1,
        soft: 0,
    };

    /// One soft constraint penalty.
    pub const ONE_SOFT: HardMediumSoftScore = HardMediumSoftScore {
        hard: 0,
        medium: 0,
        soft: 1,
    };

    /// Creates a new HardMediumSoftScore.
    #[inline]
    pub const fn of(hard: i64, medium: i64, soft: i64) -> Self {
        HardMediumSoftScore { hard, medium, soft }
    }

    /// Creates a score with only a hard component.
    #[inline]
    pub const fn of_hard(hard: i64) -> Self {
        HardMediumSoftScore {
            hard,
            medium: 0,
            soft: 0,
        }
    }

    /// Creates a score with only a medium component.
    #[inline]
    pub const fn of_medium(medium: i64) -> Self {
        HardMediumSoftScore {
            hard: 0,
            medium,
            soft: 0,
        }
    }

    /// Creates a score with only a soft component.
    #[inline]
    pub const fn of_soft(soft: i64) -> Self {
        HardMediumSoftScore {
            hard: 0,
            medium: 0,
            soft,
        }
    }

    /// Returns the hard score component.
    #[inline]
    pub const fn hard(&self) -> i64 {
        self.hard
    }

    /// Returns the medium score component.
    #[inline]
    pub const fn medium(&self) -> i64 {
        self.medium
    }

    /// Returns the soft score component.
    #[inline]
    pub const fn soft(&self) -> i64 {
        self.soft
    }
}

impl Score for HardMediumSoftScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    #[inline]
    fn zero() -> Self {
        HardMediumSoftScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        3
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        vec![self.hard, self.medium, self.soft]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(
            levels.len(),
            3,
            "HardMediumSoftScore requires exactly 3 levels"
        );
        HardMediumSoftScore::of(levels[0], levels[1], levels[2])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        let hard = (self.hard as f64 * multiplicand).round() as i64;
        let medium = (self.medium as f64 * multiplicand).round() as i64;
        let soft = (self.soft as f64 * multiplicand).round() as i64;
        HardMediumSoftScore::of(hard, medium, soft)
    }

    fn divide(&self, divisor: f64) -> Self {
        let hard = (self.hard as f64 / divisor).round() as i64;
        let medium = (self.medium as f64 / divisor).round() as i64;
        let soft = (self.soft as f64 / divisor).round() as i64;
        HardMediumSoftScore::of(hard, medium, soft)
    }

    fn abs(&self) -> Self {
        HardMediumSoftScore::of(self.hard.abs(), self.medium.abs(), self.soft.abs())
    }
}

impl Ord for HardMediumSoftScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard.cmp(&other.hard) {
            Ordering::Equal => match self.medium.cmp(&other.medium) {
                Ordering::Equal => self.soft.cmp(&other.soft),
                other => other,
            },
            other => other,
        }
    }
}

impl PartialOrd for HardMediumSoftScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for HardMediumSoftScore {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        HardMediumSoftScore::of(
            self.hard + other.hard,
            self.medium + other.medium,
            self.soft + other.soft,
        )
    }
}

impl Sub for HardMediumSoftScore {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        HardMediumSoftScore::of(
            self.hard - other.hard,
            self.medium - other.medium,
            self.soft - other.soft,
        )
    }
}

impl Neg for HardMediumSoftScore {
    type Output = Self;

    fn neg(self) -> Self {
        HardMediumSoftScore::of(-self.hard, -self.medium, -self.soft)
    }
}

impl fmt::Debug for HardMediumSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HardMediumSoftScore({}, {}, {})",
            self.hard, self.medium, self.soft
        )
    }
}

impl fmt::Display for HardMediumSoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}hard/{}medium/{}soft",
            self.hard, self.medium, self.soft
        )
    }
}

impl ParseableScore for HardMediumSoftScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();

        // Format: "0hard/0medium/-100soft"
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return Err(ScoreParseError {
                message: format!(
                    "Invalid HardMediumSoftScore format '{}': expected 'Xhard/Ymedium/Zsoft'",
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

        let medium_str = parts[1]
            .trim()
            .strip_suffix("medium")
            .ok_or_else(|| ScoreParseError {
                message: format!("Medium score part '{}' must end with 'medium'", parts[1]),
            })?;

        let soft_str = parts[2]
            .trim()
            .strip_suffix("soft")
            .ok_or_else(|| ScoreParseError {
                message: format!("Soft score part '{}' must end with 'soft'", parts[2]),
            })?;

        let hard = hard_str.parse::<i64>().map_err(|e| ScoreParseError {
            message: format!("Invalid hard score '{}': {}", hard_str, e),
        })?;

        let medium = medium_str.parse::<i64>().map_err(|e| ScoreParseError {
            message: format!("Invalid medium score '{}': {}", medium_str, e),
        })?;

        let soft = soft_str.parse::<i64>().map_err(|e| ScoreParseError {
            message: format!("Invalid soft score '{}': {}", soft_str, e),
        })?;

        Ok(HardMediumSoftScore::of(hard, medium, soft))
    }

    fn to_string_repr(&self) -> String {
        format!("{}hard/{}medium/{}soft", self.hard, self.medium, self.soft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let score = HardMediumSoftScore::of(-2, -10, -100);
        assert_eq!(score.hard(), -2);
        assert_eq!(score.medium(), -10);
        assert_eq!(score.soft(), -100);
    }

    #[test]
    fn test_feasibility() {
        assert!(HardMediumSoftScore::of(0, -100, -1000).is_feasible());
        assert!(!HardMediumSoftScore::of(-1, 0, 0).is_feasible());
    }

    #[test]
    fn test_comparison() {
        // Hard dominates
        let s1 = HardMediumSoftScore::of(-1, 0, 0);
        let s2 = HardMediumSoftScore::of(0, -1000, -1000);
        assert!(s2 > s1);

        // Medium dominates soft
        let s3 = HardMediumSoftScore::of(0, -10, 0);
        let s4 = HardMediumSoftScore::of(0, -5, -1000);
        assert!(s4 > s3);

        // Soft comparison when others equal
        let s5 = HardMediumSoftScore::of(0, 0, -100);
        let s6 = HardMediumSoftScore::of(0, 0, -50);
        assert!(s6 > s5);
    }

    #[test]
    fn test_arithmetic() {
        let s1 = HardMediumSoftScore::of(-1, -10, -100);
        let s2 = HardMediumSoftScore::of(-1, -5, -50);

        assert_eq!(s1 + s2, HardMediumSoftScore::of(-2, -15, -150));
        assert_eq!(s1 - s2, HardMediumSoftScore::of(0, -5, -50));
        assert_eq!(-s1, HardMediumSoftScore::of(1, 10, 100));
    }

    #[test]
    fn test_parse() {
        assert_eq!(
            HardMediumSoftScore::parse("0hard/-10medium/-100soft").unwrap(),
            HardMediumSoftScore::of(0, -10, -100)
        );
    }

    #[test]
    fn test_display() {
        let score = HardMediumSoftScore::of(-1, -10, -100);
        assert_eq!(format!("{}", score), "-1hard/-10medium/-100soft");
    }
}
