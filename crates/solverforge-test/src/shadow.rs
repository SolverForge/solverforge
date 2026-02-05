//! Shadow variable test fixtures.
//!
//! Provides a solution type for testing shadow variable infrastructure.
//! The `ShadowVariableSupport` trait is defined in `solverforge-scoring`,
//! so tests must implement it there.
//!
//! # Example
//!
//! ```
//! use solverforge_test::shadow::ShadowSolution;
//!
//! let solution = ShadowSolution::new(vec![10, 20, 30]);
//! assert_eq!(solution.values, vec![10, 20, 30]);
//! assert_eq!(solution.cached_sum, 0); // Not yet computed
//! ```

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;

/// A solution with a shadow variable for testing.
///
/// Contains:
/// - `values`: Planning variables (a vector of integers)
/// - `cached_sum`: Shadow variable (sum of all values, updated by shadow logic)
/// - `score`: The solution's score
#[derive(Clone, Debug)]
pub struct ShadowSolution {
    pub values: Vec<i32>,
    /// Shadow variable: cached sum of all values.
    pub cached_sum: i32,
    pub score: Option<SimpleScore>,
}

impl ShadowSolution {
    /// Creates a new shadow solution with the given values.
    ///
    /// The cached_sum is initialized to 0 and will be updated
    /// when shadow variables are triggered.
    pub fn new(values: Vec<i32>) -> Self {
        Self {
            values,
            cached_sum: 0,
            score: None,
        }
    }

    /// Creates a shadow solution with pre-computed cached_sum.
    pub fn with_cached_sum(values: Vec<i32>, cached_sum: i32) -> Self {
        Self {
            values,
            cached_sum,
            score: None,
        }
    }
}

impl Default for ShadowSolution {
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl PlanningSolution for ShadowSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_solution_creation() {
        let s1 = ShadowSolution::new(vec![1, 2, 3]);
        assert_eq!(s1.values, vec![1, 2, 3]);
        assert_eq!(s1.cached_sum, 0);
        assert!(s1.score.is_none());

        let s2 = ShadowSolution::with_cached_sum(vec![1, 2, 3], 6);
        assert_eq!(s2.cached_sum, 6);
    }

    #[test]
    fn test_default() {
        let s = ShadowSolution::default();
        assert!(s.values.is_empty());
        assert_eq!(s.cached_sum, 0);
    }
}
