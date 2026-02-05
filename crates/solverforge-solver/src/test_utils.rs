//! Test utilities for solverforge-solver
//!
//! Provides common test fixtures used across the crate's test modules.
//! Re-exports types from solverforge-test and adds solver-specific helpers.

use crate::scope::SolverScope;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
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
    pub score: Option<SimpleScore>,
}

impl TestSolution {
    /// Creates a new test solution with no score.
    pub fn new() -> Self {
        Self { score: None }
    }

    /// Creates a test solution with the given score.
    pub fn with_score(score: SimpleScore) -> Self {
        Self { score: Some(score) }
    }
}

impl Default for TestSolution {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanningSolution for TestSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

// Type aliases for backward compatibility
pub type MinimalSolution = TestSolution;
pub type DummySolution = TestSolution;

/// Type alias for a SimpleScoreDirector with a function pointer calculator.
pub type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

/// A zero-returning calculator function.
pub fn zero_calculator(_: &TestSolution) -> SimpleScore {
    SimpleScore::of(0)
}

/// Creates a SolutionDescriptor for TestSolution.
pub fn create_minimal_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
}

/// Creates a SimpleScoreDirector for TestSolution with a zero calculator.
pub fn create_minimal_director() -> TestDirector {
    let solution = TestSolution::new();
    let descriptor = create_minimal_descriptor();
    SimpleScoreDirector::with_calculator(
        solution,
        descriptor,
        zero_calculator as fn(&TestSolution) -> SimpleScore,
    )
}

// ============================================================================
// N-Queens director factories (solver-specific, using solverforge-scoring)
// ============================================================================

/// Creates a SimpleScoreDirector for N-Queens with queens at the specified rows.
pub fn create_nqueens_director(
    rows: &[i64],
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = NQueensSolution::with_rows(rows);
    let descriptor = create_nqueens_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}

/// Creates a SimpleScoreDirector for N-Queens with n uninitialized queens.
pub fn create_simple_nqueens_director(
    n: usize,
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let solution = NQueensSolution::uninitialized(n);
    let descriptor = create_nqueens_descriptor();
    SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts)
}

// ============================================================================
// SolverScope-specific helpers
// ============================================================================

/// Creates a SolverScope with the default zero calculator.
pub fn create_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = create_minimal_descriptor();
    let director = SimpleScoreDirector::with_calculator(
        TestSolution::new(),
        desc,
        zero_calculator as fn(&TestSolution) -> SimpleScore,
    );
    SolverScope::new(director)
}

/// Alias for `create_scope` for backward compatibility.
pub fn create_test_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    create_scope()
}

/// Creates a SolverScope with a fixed score that will be returned by the calculator.
/// The best solution is automatically updated.
pub fn create_scope_with_score(
    score: SimpleScore,
) -> SolverScope<
    'static,
    TestSolution,
    SimpleScoreDirector<TestSolution, impl Fn(&TestSolution) -> SimpleScore>,
> {
    let desc = create_minimal_descriptor();
    let score_clone = score;
    let director =
        SimpleScoreDirector::with_calculator(TestSolution::with_score(score), desc, move |_| {
            score_clone
        });
    let mut scope = SolverScope::new(director);
    scope.update_best_solution();
    scope
}

/// Alias for `create_scope_with_score` for backward compatibility.
pub fn create_test_scope_with_score(
    score: SimpleScore,
) -> SolverScope<
    'static,
    TestSolution,
    SimpleScoreDirector<TestSolution, impl Fn(&TestSolution) -> SimpleScore>,
> {
    create_scope_with_score(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solution_creation() {
        let s1 = TestSolution::new();
        assert!(s1.score.is_none());

        let s2 = TestSolution::with_score(SimpleScore::of(-5));
        assert_eq!(s2.score, Some(SimpleScore::of(-5)));
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
        let scope = create_scope_with_score(SimpleScore::of(-10));
        assert!(scope.best_solution().is_some());
        assert_eq!(scope.best_score(), Some(&SimpleScore::of(-10)));
    }

    #[test]
    fn test_create_test_scope_with_score_alias() {
        let scope = create_test_scope_with_score(SimpleScore::of(-10));
        assert!(scope.best_solution().is_some());
        assert_eq!(scope.best_score(), Some(&SimpleScore::of(-10)));
    }

    #[test]
    fn test_zero_calculator() {
        let solution = TestSolution::with_score(SimpleScore::of(100));
        assert_eq!(zero_calculator(&solution), SimpleScore::of(0));
    }
}
