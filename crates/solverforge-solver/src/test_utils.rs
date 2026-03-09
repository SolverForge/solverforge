//! Test utilities for solverforge-solver
//!
//! Provides common test fixtures used across the crate's test modules.
//! Re-exports types from solverforge-test and adds solver-specific helpers.

use crate::scope::SolverScope;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::director::score_director::ScoreDirector;
use std::any::TypeId;

// Re-export N-Queens test infrastructure from solverforge-test (data types only)
pub use solverforge_test::nqueens::{
    calculate_conflicts, create_nqueens_descriptor, get_queen_row, set_queen_row, NQueensSolution,
    Queen,
};

// ============================================================================
// TestSolution - a minimal solution type for solver tests
// ============================================================================

/// A minimal test solution with just a score field.
///
/// This is useful for testing components like termination conditions
/// that only need to track score, not entities.
#[derive(Clone, Debug)]
pub struct TestSolution {
    pub score: Option<SoftScore>,
}

impl TestSolution {
    /// Creates a new test solution with no score.
    pub fn new() -> Self {
        Self { score: None }
    }

    /// Creates a test solution with the given score.
    pub fn with_score(score: SoftScore) -> Self {
        Self { score: Some(score) }
    }
}

impl Default for TestSolution {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

/// Type alias for a ScoreDirector with empty constraint set.
pub type TestDirector = ScoreDirector<TestSolution, ()>;

/// Creates a SolutionDescriptor for TestSolution.
pub fn create_minimal_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
}

/// Creates a ScoreDirector for TestSolution with empty constraints.
pub fn create_minimal_director() -> TestDirector {
    let solution = TestSolution::new();
    let descriptor = create_minimal_descriptor();
    ScoreDirector::simple(solution, descriptor, |_, _| 0)
}

// ============================================================================
// N-Queens director factories (solver-specific, using solverforge-scoring)
// ============================================================================

/// Creates a ScoreDirector for N-Queens with queens at the specified rows.
pub fn create_nqueens_director(rows: &[i64]) -> ScoreDirector<NQueensSolution, ()> {
    let solution = NQueensSolution::with_rows(rows);
    let descriptor = create_nqueens_descriptor();
    ScoreDirector::simple(solution, descriptor, |s, _| s.queens.len())
}

/// Creates a ScoreDirector for N-Queens with n uninitialized queens.
pub fn create_simple_nqueens_director(n: usize) -> ScoreDirector<NQueensSolution, ()> {
    let solution = NQueensSolution::uninitialized(n);
    let descriptor = create_nqueens_descriptor();
    ScoreDirector::simple(solution, descriptor, |s, _| s.queens.len())
}

// ============================================================================
// SolverScope-specific helpers
// ============================================================================

/// Creates a SolverScope with the default zero calculator.
pub fn create_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = create_minimal_descriptor();
    let director = ScoreDirector::simple(TestSolution::new(), desc, |_, _| 0);
    SolverScope::new(director)
}

/// Alias for `create_scope` for backward compatibility.
pub fn create_test_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    create_scope()
}

/// Creates a SolverScope with a fixed score.
///
/// The score is set directly on the solution — no calculator is used.
pub fn create_scope_with_score(
    score: SoftScore,
) -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = create_minimal_descriptor();
    let solution = TestSolution::with_score(score);
    let director = ScoreDirector::simple(solution.clone(), desc, |_, _| 0);
    let mut scope = SolverScope::new(director);
    scope.set_best_solution(solution, score);
    scope
}

/// Alias for `create_scope_with_score` for backward compatibility.
pub fn create_test_scope_with_score(
    score: SoftScore,
) -> SolverScope<'static, TestSolution, TestDirector> {
    create_scope_with_score(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solution_creation() {
        let s1 = TestSolution::new();
        assert!(s1.score.is_none());

        let s2 = TestSolution::with_score(SoftScore::of(-5));
        assert_eq!(s2.score, Some(SoftScore::of(-5)));
    }

    #[test]
    fn test_create_scope() {
        let scope = create_scope();
        assert_eq!(scope.total_step_count(), 0);
    }

    #[test]
    fn test_create_test_scope_alias() {
        let scope = create_test_scope();
        assert_eq!(scope.total_step_count(), 0);
    }

    #[test]
    fn test_create_scope_with_score() {
        let scope = create_scope_with_score(SoftScore::of(-10));
        assert!(scope.best_solution().is_some());
        assert_eq!(scope.best_score(), Some(&SoftScore::of(-10)));
    }

    #[test]
    fn test_create_test_scope_with_score_alias() {
        let scope = create_test_scope_with_score(SoftScore::of(-10));
        assert!(scope.best_solution().is_some());
        assert_eq!(scope.best_score(), Some(&SoftScore::of(-10)));
    }
}
