//! BendableScore - Runtime-configurable multi-level score

use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Neg, Sub};

use super::traits::{ParseableScore, Score, ScoreParseError};

/// A score with a configurable number of hard and soft levels.
///
/// Unlike `HardSoftScore`, the number of levels is determined at runtime.
/// This is useful when the constraint structure varies between problem instances.
///
/// # Examples
///
/// ```
/// use solverforge_core::score::{BendableScore, Score};
///
/// // Create a score with 2 hard levels and 3 soft levels
/// let score = BendableScore::of(vec![-1, -2], vec![-10, -20, -30]);
///
/// assert_eq!(score.hard_levels_count(), 2);
/// assert_eq!(score.soft_levels_count(), 3);
/// assert!(!score.is_feasible());  // Negative hard scores
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BendableScore {
    hard_scores: Vec<i64>,
    soft_scores: Vec<i64>,
}

impl BendableScore {
    /// Creates a new BendableScore with the given hard and soft score vectors.
    pub fn of(hard_scores: Vec<i64>, soft_scores: Vec<i64>) -> Self {
        BendableScore {
            hard_scores,
            soft_scores,
        }
    }

    /// Creates a zero score with the specified number of levels.
    pub fn zero_with_levels(hard_levels: usize, soft_levels: usize) -> Self {
        BendableScore {
            hard_scores: vec![0; hard_levels],
            soft_scores: vec![0; soft_levels],
        }
    }

    /// Returns the number of hard score levels.
    pub fn hard_levels_count(&self) -> usize {
        self.hard_scores.len()
    }

    /// Returns the number of soft score levels.
    pub fn soft_levels_count(&self) -> usize {
        self.soft_scores.len()
    }

    /// Returns the hard score at the given level.
    ///
    /// # Panics
    /// Panics if the level is out of bounds.
    pub fn hard_score(&self, level: usize) -> i64 {
        self.hard_scores[level]
    }

    /// Returns the soft score at the given level.
    ///
    /// # Panics
    /// Panics if the level is out of bounds.
    pub fn soft_score(&self, level: usize) -> i64 {
        self.soft_scores[level]
    }

    /// Returns all hard scores as a slice.
    pub fn hard_scores(&self) -> &[i64] {
        &self.hard_scores
    }

    /// Returns all soft scores as a slice.
    pub fn soft_scores(&self) -> &[i64] {
        &self.soft_scores
    }

    /// Creates a score with a single hard level penalty at the given index.
    pub fn one_hard(hard_levels: usize, soft_levels: usize, level: usize) -> Self {
        let mut hard_scores = vec![0; hard_levels];
        hard_scores[level] = 1;
        BendableScore {
            hard_scores,
            soft_scores: vec![0; soft_levels],
        }
    }

    /// Creates a score with a single soft level penalty at the given index.
    pub fn one_soft(hard_levels: usize, soft_levels: usize, level: usize) -> Self {
        let mut soft_scores = vec![0; soft_levels];
        soft_scores[level] = 1;
        BendableScore {
            hard_scores: vec![0; hard_levels],
            soft_scores,
        }
    }

    fn ensure_compatible(&self, other: &Self) {
        assert_eq!(
            self.hard_scores.len(),
            other.hard_scores.len(),
            "Incompatible hard levels: {} vs {}",
            self.hard_scores.len(),
            other.hard_scores.len()
        );
        assert_eq!(
            self.soft_scores.len(),
            other.soft_scores.len(),
            "Incompatible soft levels: {} vs {}",
            self.soft_scores.len(),
            other.soft_scores.len()
        );
    }
}

impl Default for BendableScore {
    fn default() -> Self {
        // Default to 1 hard + 1 soft level (like HardSoftScore)
        BendableScore::zero_with_levels(1, 1)
    }
}

impl Score for BendableScore {
    fn is_feasible(&self) -> bool {
        self.hard_scores.iter().all(|&s| s >= 0)
    }

    fn zero() -> Self {
        BendableScore::default()
    }

    fn levels_count() -> usize {
        // This is a bit awkward for BendableScore since levels are runtime-determined
        // Return 0 to indicate "variable"
        0
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        let mut levels = self.hard_scores.clone();
        levels.extend(self.soft_scores.iter());
        levels
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        // Assume half hard, half soft if not otherwise specified
        let mid = levels.len() / 2;
        BendableScore::of(levels[..mid].to_vec(), levels[mid..].to_vec())
    }

    fn multiply(&self, multiplicand: f64) -> Self {
        BendableScore {
            hard_scores: self
                .hard_scores
                .iter()
                .map(|&s| (s as f64 * multiplicand).round() as i64)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .map(|&s| (s as f64 * multiplicand).round() as i64)
                .collect(),
        }
    }

    fn divide(&self, divisor: f64) -> Self {
        BendableScore {
            hard_scores: self
                .hard_scores
                .iter()
                .map(|&s| (s as f64 / divisor).round() as i64)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .map(|&s| (s as f64 / divisor).round() as i64)
                .collect(),
        }
    }

    fn abs(&self) -> Self {
        BendableScore {
            hard_scores: self.hard_scores.iter().map(|&s| s.abs()).collect(),
            soft_scores: self.soft_scores.iter().map(|&s| s.abs()).collect(),
        }
    }
}

impl Ord for BendableScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ensure_compatible(other);

        // Compare hard scores first (highest priority first)
        for (a, b) in self.hard_scores.iter().zip(other.hard_scores.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        // Then compare soft scores
        for (a, b) in self.soft_scores.iter().zip(other.soft_scores.iter()) {
            match a.cmp(b) {
                Ordering::Equal => continue,
                other => return other,
            }
        }

        Ordering::Equal
    }
}

impl PartialOrd for BendableScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Add for BendableScore {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self.ensure_compatible(&other);
        BendableScore {
            hard_scores: self
                .hard_scores
                .iter()
                .zip(other.hard_scores.iter())
                .map(|(a, b)| a + b)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .zip(other.soft_scores.iter())
                .map(|(a, b)| a + b)
                .collect(),
        }
    }
}

impl Sub for BendableScore {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        self.ensure_compatible(&other);
        BendableScore {
            hard_scores: self
                .hard_scores
                .iter()
                .zip(other.hard_scores.iter())
                .map(|(a, b)| a - b)
                .collect(),
            soft_scores: self
                .soft_scores
                .iter()
                .zip(other.soft_scores.iter())
                .map(|(a, b)| a - b)
                .collect(),
        }
    }
}

impl Neg for BendableScore {
    type Output = Self;

    fn neg(self) -> Self {
        BendableScore {
            hard_scores: self.hard_scores.iter().map(|&s| -s).collect(),
            soft_scores: self.soft_scores.iter().map(|&s| -s).collect(),
        }
    }
}

impl fmt::Debug for BendableScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BendableScore(hard: {:?}, soft: {:?})",
            self.hard_scores, self.soft_scores
        )
    }
}

impl fmt::Display for BendableScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: "[0/0]hard/[-10/-20/-30]soft"
        let hard_str: Vec<String> = self.hard_scores.iter().map(|s| s.to_string()).collect();
        let soft_str: Vec<String> = self.soft_scores.iter().map(|s| s.to_string()).collect();

        write!(
            f,
            "[{}]hard/[{}]soft",
            hard_str.join("/"),
            soft_str.join("/")
        )
    }
}

impl ParseableScore for BendableScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();

        // Format: "[0/0]hard/[-10/-20/-30]soft"
        let parts: Vec<&str> = s.split("hard/").collect();
        if parts.len() != 2 {
            return Err(ScoreParseError {
                message: format!(
                    "Invalid BendableScore format '{}': expected '[...]hard/[...]soft'",
                    s
                ),
            });
        }

        let hard_part = parts[0]
            .trim()
            .strip_prefix('[')
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| ScoreParseError {
                message: format!("Hard score part '{}' must be wrapped in brackets", parts[0]),
            })?;

        let soft_part = parts[1]
            .trim()
            .strip_suffix("soft")
            .and_then(|s| s.strip_prefix('['))
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| ScoreParseError {
                message: format!(
                    "Soft score part '{}' must be wrapped in brackets and end with 'soft'",
                    parts[1]
                ),
            })?;

        let hard_scores: Result<Vec<i64>, _> = hard_part
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.trim().parse::<i64>().map_err(|e| ScoreParseError {
                    message: format!("Invalid hard score '{}': {}", s, e),
                })
            })
            .collect();

        let soft_scores: Result<Vec<i64>, _> = soft_part
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.trim().parse::<i64>().map_err(|e| ScoreParseError {
                    message: format!("Invalid soft score '{}': {}", s, e),
                })
            })
            .collect();

        Ok(BendableScore::of(hard_scores?, soft_scores?))
    }

    fn to_string_repr(&self) -> String {
        format!("{}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let score = BendableScore::of(vec![-1, -2], vec![-10, -20, -30]);
        assert_eq!(score.hard_levels_count(), 2);
        assert_eq!(score.soft_levels_count(), 3);
        assert_eq!(score.hard_score(0), -1);
        assert_eq!(score.hard_score(1), -2);
        assert_eq!(score.soft_score(2), -30);
    }

    #[test]
    fn test_feasibility() {
        let feasible = BendableScore::of(vec![0, 0], vec![-10, -20]);
        let infeasible = BendableScore::of(vec![0, -1], vec![0, 0]);

        assert!(feasible.is_feasible());
        assert!(!infeasible.is_feasible());
    }

    #[test]
    fn test_comparison() {
        // First hard level dominates
        let s1 = BendableScore::of(vec![-1, 0], vec![0]);
        let s2 = BendableScore::of(vec![0, -100], vec![-1000]);
        assert!(s2 > s1);

        // Second hard level matters when first is equal
        let s3 = BendableScore::of(vec![0, -10], vec![0]);
        let s4 = BendableScore::of(vec![0, -5], vec![-100]);
        assert!(s4 > s3);
    }

    #[test]
    fn test_arithmetic() {
        let s1 = BendableScore::of(vec![-1], vec![-10, -20]);
        let s2 = BendableScore::of(vec![-2], vec![-5, -10]);

        let sum = s1.clone() + s2.clone();
        assert_eq!(sum.hard_scores(), &[-3]);
        assert_eq!(sum.soft_scores(), &[-15, -30]);

        let neg = -s1;
        assert_eq!(neg.hard_scores(), &[1]);
        assert_eq!(neg.soft_scores(), &[10, 20]);
    }

    #[test]
    fn test_parse() {
        let score = BendableScore::parse("[0/-1]hard/[-10/-20/-30]soft").unwrap();
        assert_eq!(score.hard_scores(), &[0, -1]);
        assert_eq!(score.soft_scores(), &[-10, -20, -30]);
    }

    #[test]
    fn test_display() {
        let score = BendableScore::of(vec![0, -1], vec![-10, -20]);
        assert_eq!(format!("{}", score), "[0/-1]hard/[-10/-20]soft");
    }
}
