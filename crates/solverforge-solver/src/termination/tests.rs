// Integration tests for termination conditions.

use super::*;
use crate::scope::SolverScope;
use crate::test_utils::{create_scope, create_scope_with_score, TestSolution};
use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;
use std::any::TypeId;
use std::time::Duration;

#[test]
fn test_step_count_termination() {
    let mut scope = create_scope();
    let mut term = StepCountTermination::new(3);

    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_not_reached() {
    let scope = create_scope_with_score(SoftScore::of(-5));
    let mut term: BestScoreTermination<SoftScore> = BestScoreTermination::new(SoftScore::of(0));

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_reached() {
    let scope = create_scope_with_score(SoftScore::of(0));
    let mut term: BestScoreTermination<SoftScore> = BestScoreTermination::new(SoftScore::of(0));

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_exceeded() {
    let scope = create_scope_with_score(SoftScore::of(5));
    let mut term: BestScoreTermination<SoftScore> = BestScoreTermination::new(SoftScore::of(0));

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_no_score() {
    let scope = create_scope();
    let mut term: BestScoreTermination<SoftScore> = BestScoreTermination::new(SoftScore::of(0));

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_unimproved_step_count_termination() {
    let mut scope = create_scope_with_score(SoftScore::of(-10));
    let mut term = UnimprovedStepCountTermination::<TestSolution>::new(3);

    // Initial check - not terminated
    assert!(!term.is_terminated(&scope));

    // Step 1 - no improvement
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));

    // Step 2 - no improvement
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));

    // Step 3 - no improvement, should terminate
    scope.increment_step_count();
    assert!(term.is_terminated(&scope));
}

#[test]
fn test_unimproved_step_count_termination_with_improvement() {
    let desc = SolutionDescriptor::new("Test", TypeId::of::<TestSolution>());
    let director = ScoreDirector::simple(
        TestSolution {
            score: Some(SoftScore::of(-10)),
        },
        desc,
        |_, _| 0,
    );
    let mut scope = SolverScope::new(director);
    scope.set_best_solution(
        TestSolution {
            score: Some(SoftScore::of(-10)),
        },
        SoftScore::of(-10),
    );

    let mut term = UnimprovedStepCountTermination::<TestSolution>::new(3);

    // Initial check
    assert!(!term.is_terminated(&scope));

    // Two steps without improvement
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));

    // Simulate improvement by setting a better best score
    scope.set_best_solution(
        TestSolution {
            score: Some(SoftScore::of(-5)),
        },
        SoftScore::of(-5),
    );
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope)); // Reset counter due to improvement

    // Now count again from improvement
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    assert!(term.is_terminated(&scope)); // 3 steps since improvement
}

#[test]
fn test_or_termination() {
    let mut scope = create_scope_with_score(SoftScore::of(-10));

    // Either: best score >= 0 OR step count >= 3
    let mut term = OrTermination::new((
        BestScoreTermination::new(SoftScore::of(0)),
        StepCountTermination::new(3),
    ));

    // Neither condition met
    assert!(!term.is_terminated(&scope));

    // Step count condition met
    scope.increment_step_count();
    scope.increment_step_count();
    scope.increment_step_count();
    assert!(term.is_terminated(&scope));
}

#[test]
fn test_unimproved_time_termination_no_score() {
    let scope = create_scope();
    let mut term = UnimprovedTimeTermination::<TestSolution>::new(Duration::from_millis(10));

    // No score yet, should not terminate
    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_unimproved_time_termination_initial_score() {
    let scope = create_scope_with_score(SoftScore::of(-10));
    let mut term = UnimprovedTimeTermination::<TestSolution>::new(Duration::from_millis(100));

    // First check records the score, should not terminate
    assert!(!term.is_terminated(&scope));
}
