//! HardSoftDecimalScore - Two-level score with i64 precision and ×100000 scaling
//!
//! This score type represents a decimal score without heap allocation.
//! Internal values are scaled by 100000 to provide 5 decimal places of precision,
//! matching Timefold's BigDecimal score display format.

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use super::traits::{ParseableScore, Score, ScoreParseError};

/// Scale factor for 5 decimal places of precision (matching Timefold).
const SCALE: i64 = 100_000;

/// A score with separate hard and soft constraint levels, using i64 with ×100000 scaling.
///
/// This provides 5 decimal places of precision (matching Timefold's BigDecimal display)
/// while maintaining zero heap allocation and full type safety.
///
/// Internal values are stored pre-scaled. Use [`of`](Self::of) for unscaled input
/// or [`of_scaled`](Self::of_scaled) for pre-scaled values.
///
/// # Examples
///
/// ```
/// use solverforge_core::{HardSoftDecimalScore, Score};
///
/// // Create from unscaled values (automatically multiplied by 100000)
/// let score1 = HardSoftDecimalScore::of(-1, -100);
/// assert_eq!(score1.hard_scaled(), -100000);
/// assert_eq!(score1.soft_scaled(), -10000000);
///
/// // Create from pre-scaled values (for minute-based penalties)
/// let score2 = HardSoftDecimalScore::of_scaled(-3050000, 0);  // -30.5 hard
/// assert!(!score2.is_feasible());
///
/// // Display shows values (trailing zeros stripped)
/// let score3 = HardSoftDecimalScore::of_scaled(-150000, -250000);
/// assert_eq!(format!("{}", score3), "-1.5hard/-2.5soft");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HardSoftDecimalScore {
    hard: i64,
    soft: i64,
}

impl HardSoftDecimalScore {
    /// The zero score.
    pub const ZERO: HardSoftDecimalScore = HardSoftDecimalScore { hard: 0, soft: 0 };

    /// One hard constraint penalty (scaled).
    pub const ONE_HARD: HardSoftDecimalScore = HardSoftDecimalScore {
        hard: SCALE,
        soft: 0,
    };

    /// One soft constraint penalty (scaled).
    pub const ONE_SOFT: HardSoftDecimalScore = HardSoftDecimalScore {
        hard: 0,
        soft: SCALE,
    };

    /// Creates a new score from unscaled values.
    ///
    /// The values are automatically multiplied by 100000.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_core::HardSoftDecimalScore;
    ///
    /// let score = HardSoftDecimalScore::of(-2, -100);
    /// assert_eq!(score.hard_scaled(), -200000);
    /// assert_eq!(score.soft_scaled(), -10000000);
    /// ```
    #[inline]
    pub const fn of(hard: i64, soft: i64) -> Self {
        HardSoftDecimalScore {
            hard: hard * SCALE,
            soft: soft * SCALE,
        }
    }

    /// Creates a new score from pre-scaled values.
    ///
    /// Use this for minute-based penalties where precision matters.
    ///
    /// # Examples
    ///
    /// ```
    /// use solverforge_core::HardSoftDecimalScore;
    ///
    /// // -30.5 hard constraint (overlap of 30.5 minutes)
    /// let score = HardSoftDecimalScore::of_scaled(-3050000, 0);
    /// assert_eq!(score.hard_scaled(), -3050000);
    /// ```
    #[inline]
    pub const fn of_scaled(hard: i64, soft: i64) -> Self {
        HardSoftDecimalScore { hard, soft }
    }

    /// Creates a score with only a hard component (unscaled input).
    #[inline]
    pub const fn of_hard(hard: i64) -> Self {
        HardSoftDecimalScore {
            hard: hard * SCALE,
            soft: 0,
        }
    }

    /// Creates a score with only a soft component (unscaled input).
    #[inline]
    pub const fn of_soft(soft: i64) -> Self {
        HardSoftDecimalScore {
            hard: 0,
            soft: soft * SCALE,
        }
    }

    /// Creates a score with only a hard component (pre-scaled input).
    #[inline]
    pub const fn of_hard_scaled(hard: i64) -> Self {
        HardSoftDecimalScore { hard, soft: 0 }
    }

    /// Creates a score with only a soft component (pre-scaled input).
    #[inline]
    pub const fn of_soft_scaled(soft: i64) -> Self {
        HardSoftDecimalScore { hard: 0, soft }
    }

    /// Returns the scaled hard score component.
    #[inline]
    pub const fn hard_scaled(&self) -> i64 {
        self.hard
    }

    /// Returns the scaled soft score component.
    #[inline]
    pub const fn soft_scaled(&self) -> i64 {
        self.soft
    }

    /// Returns the hard score as a new HardSoftDecimalScore.
    pub const fn hard_score(&self) -> HardSoftDecimalScore {
        HardSoftDecimalScore::of_scaled(self.hard, 0)
    }

    /// Returns the soft score as a new HardSoftDecimalScore.
    pub const fn soft_score(&self) -> HardSoftDecimalScore {
        HardSoftDecimalScore::of_scaled(0, self.soft)
    }

    /// Returns true if this score has a non-zero hard component.
    ///
    /// Used by constraint streams to determine if a weight represents
    /// a hard or soft constraint.
    #[inline]
    pub const fn has_hard_component(&self) -> bool {
        self.hard != 0
    }
}

impl Score for HardSoftDecimalScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.hard >= 0
    }

    #[inline]
    fn zero() -> Self {
        HardSoftDecimalScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        2
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        vec![self.hard, self.soft]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(
            levels.len(),
            2,
            "HardSoftDecimalScore requires exactly 2 levels"
        );
        HardSoftDecimalScore::of_scaled(levels[0], levels[1])
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        // Multiply scaled values directly, round to nearest integer
        let hard = (self.hard as f64 * multiplicand).round() as i64;
        let soft = (self.soft as f64 * multiplicand).round() as i64;
        HardSoftDecimalScore::of_scaled(hard, soft)
    }

    fn divide(&self, divisor: f64) -> Self {
        let hard = (self.hard as f64 / divisor).round() as i64;
        let soft = (self.soft as f64 / divisor).round() as i64;
        HardSoftDecimalScore::of_scaled(hard, soft)
    }

    fn abs(&self) -> Self {
        HardSoftDecimalScore::of_scaled(self.hard.abs(), self.soft.abs())
    }
}

impl Ord for HardSoftDecimalScore {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.hard.cmp(&other.hard) {
            Ordering::Equal => self.soft.cmp(&other.soft),
            other => other,
        }
    }
}

impl PartialOrd for HardSoftDecimalScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for HardSoftDecimalScore {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        HardSoftDecimalScore::of_scaled(self.hard + other.hard, self.soft + other.soft)
    }
}

impl Sub for HardSoftDecimalScore {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        HardSoftDecimalScore::of_scaled(self.hard - other.hard, self.soft - other.soft)
    }
}

impl Neg for HardSoftDecimalScore {
    type Output = Self;

    fn neg(self) -> Self {
        HardSoftDecimalScore::of_scaled(-self.hard, -self.soft)
    }
}

impl fmt::Debug for HardSoftDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HardSoftDecimalScore({:.3}, {:.3})",
            self.hard as f64 / SCALE as f64,
            self.soft as f64 / SCALE as f64
        )
    }
}

impl fmt::Display for HardSoftDecimalScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn format_score_part(scaled: i64) -> String {
            if scaled % SCALE == 0 {
                // Integer value, no decimals needed
                (scaled / SCALE).to_string()
            } else {
                // Has decimal part - format with precision and strip trailing zeros
                let value = scaled as f64 / SCALE as f64;
                let formatted = format!("{:.6}", value);
                formatted
                    .trim_end_matches('0')
                    .trim_end_matches('.')
                    .to_string()
            }
        }
        write!(
            f,
            "{}hard/{}soft",
            format_score_part(self.hard),
            format_score_part(self.soft)
        )
    }
}

impl ParseableScore for HardSoftDecimalScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();

        // Format: "0.000hard/-100.500soft" or "-1hard/0soft"
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(ScoreParseError {
                message: format!(
                    "Invalid HardSoftDecimalScore format '{}': expected 'Xhard/Ysoft'",
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

        let hard_float = hard_str.parse::<f64>().map_err(|e| ScoreParseError {
            message: format!("Invalid hard score '{}': {}", hard_str, e),
        })?;

        let soft_float = soft_str.parse::<f64>().map_err(|e| ScoreParseError {
            message: format!("Invalid soft score '{}': {}", soft_str, e),
        })?;

        // Convert to scaled integers
        let hard = (hard_float * SCALE as f64).round() as i64;
        let soft = (soft_float * SCALE as f64).round() as i64;

        Ok(HardSoftDecimalScore::of_scaled(hard, soft))
    }

    fn to_string_repr(&self) -> String {
        format!("{}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation_unscaled() {
        let score = HardSoftDecimalScore::of(-2, -100);
        assert_eq!(score.hard_scaled(), -200000);
        assert_eq!(score.soft_scaled(), -10000000);
    }

    #[test]
    fn test_creation_scaled() {
        let score = HardSoftDecimalScore::of_scaled(-30500, -208250);
        assert_eq!(score.hard_scaled(), -30500);
        assert_eq!(score.soft_scaled(), -208250);
    }

    #[test]
    fn test_feasibility() {
        assert!(HardSoftDecimalScore::of(0, -1000).is_feasible());
        assert!(HardSoftDecimalScore::of(10, -50).is_feasible());
        assert!(!HardSoftDecimalScore::of(-1, 0).is_feasible());
        assert!(!HardSoftDecimalScore::of_scaled(-1, 0).is_feasible());
    }

    #[test]
    fn test_comparison() {
        let infeasible = HardSoftDecimalScore::of(-1, 0);
        let feasible = HardSoftDecimalScore::of(0, -1000);
        assert!(feasible > infeasible);

        let s1 = HardSoftDecimalScore::of(0, -100);
        let s2 = HardSoftDecimalScore::of(0, -50);
        assert!(s2 > s1);

        let s3 = HardSoftDecimalScore::of(-2, 0);
        let s4 = HardSoftDecimalScore::of(-1, -1000);
        assert!(s4 > s3);
    }

    #[test]
    fn test_arithmetic() {
        let s1 = HardSoftDecimalScore::of(-1, -100);
        let s2 = HardSoftDecimalScore::of(-1, -50);

        assert_eq!(s1 + s2, HardSoftDecimalScore::of(-2, -150));
        assert_eq!(s1 - s2, HardSoftDecimalScore::of(0, -50));
        assert_eq!(-s1, HardSoftDecimalScore::of(1, 100));
    }

    #[test]
    fn test_arithmetic_scaled() {
        let s1 = HardSoftDecimalScore::of_scaled(-1500, -100500);
        let s2 = HardSoftDecimalScore::of_scaled(-500, -50250);

        let sum = s1 + s2;
        assert_eq!(sum.hard_scaled(), -2000);
        assert_eq!(sum.soft_scaled(), -150750);
    }

    #[test]
    fn test_parse_integer() {
        let score = HardSoftDecimalScore::parse("0hard/-100soft").unwrap();
        assert_eq!(score.hard_scaled(), 0);
        assert_eq!(score.soft_scaled(), -10000000);
    }

    #[test]
    fn test_parse_decimal() {
        let score = HardSoftDecimalScore::parse("-30.5hard/-208.25soft").unwrap();
        assert_eq!(score.hard_scaled(), -3050000);
        assert_eq!(score.soft_scaled(), -20825000);
    }

    #[test]
    fn test_display() {
        let score = HardSoftDecimalScore::of_scaled(-3050000, -20825000);
        assert_eq!(format!("{}", score), "-30.5hard/-208.25soft");
    }

    #[test]
    fn test_display_integer() {
        let score = HardSoftDecimalScore::of(-2, -100);
        assert_eq!(format!("{}", score), "-2hard/-100soft");
    }

    #[test]
    fn test_display_zero() {
        let score = HardSoftDecimalScore::ZERO;
        assert_eq!(format!("{}", score), "0hard/0soft");
    }

    #[test]
    fn test_level_numbers() {
        let score = HardSoftDecimalScore::of_scaled(-2000, -50000);
        assert_eq!(score.to_level_numbers(), vec![-2000, -50000]);
        assert_eq!(
            HardSoftDecimalScore::from_level_numbers(&[-2000, -50000]),
            score
        );
    }
}
