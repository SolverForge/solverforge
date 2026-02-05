//! Test utilities for solverforge-solver
//!
//! Provides common test fixtures used across the crate's test modules.
//! Re-exports types from solverforge-test and adds solver-specific helpers.

use crate::scope::SolverScope;
use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

// Re-export N-Queens test infrastructure from solverforge-test
pub use solverforge_test::nqueens::{
    calculate_conflicts, create_nqueens_descriptor, create_nqueens_director,
    create_simple_nqueens_director, get_queen_row, set_queen_row, NQueensSolution, Queen,
};

// Re-export minimal solution types from solverforge-test
pub use solverforge_test::minimal::{
    create_minimal_descriptor, create_minimal_director, zero_calculator, DummySolution,
    MinimalSolution, TestDirector, TestSolution,
};

// ============================================================================
// SolverScope-specific helpers (cannot be in solverforge-test due to dependency order)
// ============================================================================

/// Creates a SolverScope with the default zero calculator.
pub fn create_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
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
    let desc = SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>());
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
