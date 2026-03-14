// Core Score trait definition

use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::ops::{Add, Neg, Sub};

use super::ScoreLevel;

/// Core trait for all score types in SolverForge.
///
/// Scores represent the quality of a planning solution. They are used to:
/// - Compare solutions (better/worse/equal)
/// - Guide the optimization process
/// - Determine feasibility
///
/// All score implementations must be:
/// - Immutable (operations return new instances)
/// - Thread-safe (Send + Sync)
/// - Comparable (total ordering)
///
/// # Score Levels
///
/// Scores can have multiple levels (e.g., hard/soft constraints):
/// - Hard constraints: Must be satisfied for a solution to be feasible
/// - Soft constraints: Optimization objectives to maximize/minimize
///
/// When comparing scores, higher-priority levels are compared first.
pub trait Score:
    Copy
    + Debug
    + Display
    + Default
    + Send
    + Sync
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Add<Output = Self>
    + Sub<Output = Self>
    + Neg<Output = Self>
    + 'static
{
    /* Returns true if this score represents a feasible solution.

    A solution is feasible when all hard constraints are satisfied
    (i.e., the hard score is >= 0).
    */
    fn is_feasible(&self) -> bool;

    fn zero() -> Self;

    /* Returns the number of score levels.

    For example:
    - SoftScore: 1 level
    - HardSoftScore: 2 levels
    - HardMediumSoftScore: 3 levels
    */
    fn levels_count() -> usize;

    /* Returns the score values as a vector of i64.

    The order is from highest priority to lowest priority.
    For HardSoftScore: [hard, soft]
    */
    fn to_level_numbers(&self) -> Vec<i64>;

    /* Creates a score from level numbers.

    # Panics
    Panics if the number of levels doesn't match `levels_count()`.
    */
    fn from_level_numbers(levels: &[i64]) -> Self;

    // Multiplies this score by a scalar.
    fn multiply(&self, multiplicand: f64) -> Self;

    // Divides this score by a scalar.
    fn divide(&self, divisor: f64) -> Self;

    fn abs(&self) -> Self;

    /* Converts this score to a single f64 scalar value.

    Higher-priority levels are weighted with larger multipliers to preserve
    their dominance. Used for simulated annealing temperature calculations.
    */
    fn to_scalar(&self) -> f64;

    /* Returns the semantic label for the score level at the given index.

    Level indices follow the same order as `to_level_numbers()`:
    highest priority first.

    # Panics
    Panics if `index >= levels_count()`.
    */
    fn level_label(index: usize) -> ScoreLevel;

    /* Compares two scores, returning the ordering.

    Default implementation uses the Ord trait.
    */
    fn compare(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }

    /* Returns true if this score is better than the other score.

    In optimization, "better" typically means higher score.
    */
    fn is_better_than(&self, other: &Self) -> bool {
        self > other
    }

    // Returns true if this score is worse than the other score.
    fn is_worse_than(&self, other: &Self) -> bool {
        self < other
    }

    // Returns true if this score is equal to the other score.
    fn is_equal_to(&self, other: &Self) -> bool {
        self == other
    }

    // Returns a score with 1 at the first Hard-labeled level and 0 elsewhere.
    fn one_hard() -> Self {
        let mut levels = vec![0i64; Self::levels_count()];
        if let Some(i) =
            (0..Self::levels_count()).find(|&i| Self::level_label(i) == ScoreLevel::Hard)
        {
            levels[i] = 1;
        }
        Self::from_level_numbers(&levels)
    }

    // Returns a score with 1 at the last Soft-labeled level and 0 elsewhere.
    fn one_soft() -> Self {
        let mut levels = vec![0i64; Self::levels_count()];
        if let Some(i) = (0..Self::levels_count())
            .rev()
            .find(|&i| Self::level_label(i) == ScoreLevel::Soft)
        {
            levels[i] = 1;
        }
        Self::from_level_numbers(&levels)
    }

    // Returns a score with 1 at the first Medium-labeled level and 0 elsewhere.
    fn one_medium() -> Self {
        let mut levels = vec![0i64; Self::levels_count()];
        if let Some(i) =
            (0..Self::levels_count()).find(|&i| Self::level_label(i) == ScoreLevel::Medium)
        {
            levels[i] = 1;
        }
        Self::from_level_numbers(&levels)
    }
}

/// Marker trait for scores that can be parsed from a string.
pub trait ParseableScore: Score {
    /* Parses a score from a string representation.

    # Format
    - SoftScore: "42" or "42init"
    - HardSoftScore: "0hard/-100soft" or "-1hard/0soft"
    - HardMediumSoftScore: "0hard/0medium/-100soft"
    */
    fn parse(s: &str) -> Result<Self, ScoreParseError>;

    fn to_string_repr(&self) -> String;
}

// Error when parsing a score from string
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreParseError {
    pub message: String,
}

impl std::fmt::Display for ScoreParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Score parse error: {}", self.message)
    }
}

impl std::error::Error for ScoreParseError {}
