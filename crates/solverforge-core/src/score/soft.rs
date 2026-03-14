// SoftScore - Single-level score implementation

use std::cmp::Ordering;
use std::fmt;

use super::traits::{ParseableScore, Score, ScoreParseError};
use super::ScoreLevel;

/* A simple score with a single integer value.

This is the simplest score type, useful when there's only one
type of constraint to optimize.

# Examples

```
use solverforge_core::{SoftScore, Score};

let score1 = SoftScore::of(-5);
let score2 = SoftScore::of(-3);

assert!(score2 > score1);  // -3 is better than -5
assert!(!score1.is_feasible());  // Negative scores are not feasible
```
*/
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SoftScore {
    score: i64,
}

impl SoftScore {
    /// The zero score.
    pub const ZERO: SoftScore = SoftScore { score: 0 };

    /// A score of 1 (useful for incrementing).
    pub const ONE: SoftScore = SoftScore { score: 1 };

    #[inline]
    pub const fn of(score: i64) -> Self {
        SoftScore { score }
    }

    #[inline]
    pub const fn score(&self) -> i64 {
        self.score
    }
}

impl Score for SoftScore {
    #[inline]
    fn is_feasible(&self) -> bool {
        self.score >= 0
    }

    #[inline]
    fn zero() -> Self {
        SoftScore::ZERO
    }

    #[inline]
    fn levels_count() -> usize {
        1
    }

    fn to_level_numbers(&self) -> Vec<i64> {
        vec![self.score]
    }

    fn from_level_numbers(levels: &[i64]) -> Self {
        assert_eq!(levels.len(), 1, "SoftScore requires exactly 1 level");
        SoftScore::of(levels[0])
    }

    impl_score_scale!(SoftScore { score } => of);

    fn level_label(index: usize) -> ScoreLevel {
        match index {
            0 => ScoreLevel::Soft,
            _ => panic!("SoftScore has 1 level, got index {}", index),
        }
    }

    #[inline]
    fn to_scalar(&self) -> f64 {
        self.score as f64
    }
}

impl Ord for SoftScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.score.cmp(&other.score)
    }
}

impl_score_ops!(SoftScore { score } => of);

impl fmt::Debug for SoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SoftScore({})", self.score)
    }
}

impl fmt::Display for SoftScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.score)
    }
}

// SoftScore has custom parse logic (optional "init" suffix) so no macro.
impl ParseableScore for SoftScore {
    fn parse(s: &str) -> Result<Self, ScoreParseError> {
        let s = s.trim();
        // Remove optional "init" suffix
        let s = s.strip_suffix("init").unwrap_or(s);

        s.parse::<i64>()
            .map(SoftScore::of)
            .map_err(|e| ScoreParseError {
                message: format!("Invalid SoftScore '{}': {}", s, e),
            })
    }

    fn to_string_repr(&self) -> String {
        self.score.to_string()
    }
}

impl From<i64> for SoftScore {
    fn from(score: i64) -> Self {
        SoftScore::of(score)
    }
}
