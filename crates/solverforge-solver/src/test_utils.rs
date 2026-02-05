//! Test utilities for solverforge-solver
//!
//! Provides common test fixtures used across the crate's test modules.

use crate::scope::SolverScope;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

/// A minimal test solution with just a score field.
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

/// Type alias for a SimpleScoreDirector with a function pointer calculator.
pub type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

/// A zero-returning calculator function for TestSolution.
pub fn zero_calculator(_: &TestSolution) -> SimpleScore {
    SimpleScore::of(0)
}

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

/// Creates a SolutionDescriptor for TestSolution.
pub fn create_test_descriptor() -> SolutionDescriptor {
    SolutionDescriptor::new("TestSolution", TypeId::of::<TestSolution>())
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
    fn test_create_scope_with_score() {
        let scope = create_scope_with_score(SimpleScore::of(-10));
        assert!(scope.best_solution().is_some());
        assert_eq!(scope.best_score(), Some(&SimpleScore::of(-10)));
    }

    #[test]
    fn test_zero_calculator() {
        let solution = TestSolution::with_score(SimpleScore::of(100));
        assert_eq!(zero_calculator(&solution), SimpleScore::of(0));
    }
}
