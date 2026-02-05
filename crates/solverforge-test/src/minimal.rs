//! Minimal test solution fixtures.
//!
//! Provides a minimal solution type with only a score field, useful for testing
//! termination conditions and other components that don't need entity infrastructure.
//!
//! # Example
//!
//! ```ignore
//! use solverforge_test::minimal::{TestSolution, create_minimal_director};
//!
//! let director = create_minimal_director();
//! ```

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

/// A minimal test solution with just a score field.
///
/// This is useful for testing components like termination conditions
/// that only need to track score, not entities.
#[derive(Clone, Debug)]
pub struct MinimalSolution {
    pub score: Option<SimpleScore>,
}

impl MinimalSolution {
    /// Creates a new minimal solution with no score.
    pub fn new() -> Self {
        Self { score: None }
    }

    /// Creates a minimal solution with the given score.
    pub fn with_score(score: SimpleScore) -> Self {
        Self { score: Some(score) }
    }
}

impl Default for MinimalSolution {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningSolution for MinimalSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// Type alias for backward compatibility with existing tests.
pub type TestSolution = MinimalSolution;

/// Type alias for backward compatibility (identical to TestSolution).
pub type DummySolution = MinimalSolution;

/// Type alias for a SimpleScoreDirector with a function pointer calculator.
pub type MinimalDirector =
    SimpleScoreDirector<MinimalSolution, fn(&MinimalSolution) -> SimpleScore>;

/// Type alias for backward compatibility.
pub type TestDirector = MinimalDirector;

/// A zero-returning calculator function.
pub fn zero_calculator(_: &MinimalSolution) -> SimpleScore {
    SimpleScore::of(0)
}

/// Creates a SolutionDescriptor for MinimalSolution.
pub fn create_minimal_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("MinimalSolution", TypeId::of::<MinimalSolution>())
}

/// Creates a SimpleScoreDirector for MinimalSolution with a zero calculator.
pub fn create_minimal_director() -> MinimalDirector {
    let solution = MinimalSolution::new();
    let descriptor = create_minimal_descriptor();
    SimpleScoreDirector::with_calculator(
        solution,
        descriptor,
        zero_calculator as fn(&MinimalSolution) -> SimpleScore,
    )
}

/// Creates a SimpleScoreDirector for MinimalSolution with a fixed score.
pub fn create_minimal_director_with_score(
    score: SimpleScore,
) -> SimpleScoreDirector<MinimalSolution, impl Fn(&MinimalSolution) -> SimpleScore> {
    let solution = MinimalSolution::with_score(score);
    let descriptor = create_minimal_descriptor();
    let score_clone = score;
    SimpleScoreDirector::with_calculator(solution, descriptor, move |_| score_clone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solverforge_scoring::ScoreDirector;

    #[test]
    fn test_minimal_solution_creation() {
        let s1 = MinimalSolution::new();
        assert!(s1.score.is_none());

        let s2 = MinimalSolution::with_score(SimpleScore::of(-5));
        assert_eq!(s2.score, Some(SimpleScore::of(-5)));
    }

    #[test]
    fn test_type_aliases() {
        let _: TestSolution = MinimalSolution::new();
        let _: DummySolution = MinimalSolution::new();
    }

    #[test]
    fn test_zero_calculator() {
        let solution = MinimalSolution::with_score(SimpleScore::of(100));
        assert_eq!(zero_calculator(&solution), SimpleScore::of(0));
    }

    #[test]
    fn test_create_minimal_director() {
        let mut director = create_minimal_director();
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(0));
    }

    #[test]
    fn test_create_minimal_director_with_score() {
        let mut director = create_minimal_director_with_score(SimpleScore::of(-10));
        let score = director.calculate_score();
        assert_eq!(score, SimpleScore::of(-10));
    }
}
