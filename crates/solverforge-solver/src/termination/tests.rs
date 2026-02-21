//! Integration tests for termination conditions.

use super::*;
use crate::scope::SolverScope;
use crate::test_utils::{
    create_scope, create_scope_with_score, create_test_scope, create_test_scope_with_score,
    TestSolution,
};
use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[test]
fn test_step_count_termination() {
    let mut scope = create_test_scope();
    let term = StepCountTermination::new(3);

    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));
    scope.increment_step_count();
    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_not_reached() {
    let scope = create_test_scope_with_score(SimpleScore::of(-5));
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_reached() {
    let scope = create_test_scope_with_score(SimpleScore::of(0));
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_exceeded() {
    let scope = create_test_scope_with_score(SimpleScore::of(5));
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_no_score() {
    let scope = create_test_scope();
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_feasible_termination() {
    let scope = create_test_scope_with_score(SimpleScore::of(0));
    let term = BestScoreFeasibleTermination::<TestSolution, _>::score_at_least_zero();

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_feasible_termination_not_feasible() {
    let scope = create_test_scope_with_score(SimpleScore::of(-5));
    let term = BestScoreFeasibleTermination::<TestSolution, _>::score_at_least_zero();

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_feasible_termination_custom() {
    let scope = create_test_scope_with_score(SimpleScore::of(-3));
    // Custom feasibility: score >= -5 is considered feasible
    let term = BestScoreFeasibleTermination::<TestSolution, _>::new(|score: &SimpleScore| {
        *score >= SimpleScore::of(-5)
    });

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_unimproved_step_count_termination() {
    let mut scope = create_test_scope_with_score(SimpleScore::of(-10));
    let term = UnimprovedStepCountTermination::<TestSolution>::new(3);

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
    fn calc(_: &TestSolution) -> SimpleScore {
        SimpleScore::of(-10)
    }
    let director = SimpleScoreDirector::with_calculator(
        TestSolution {
            score: Some(SimpleScore::of(-10)),
        },
        desc,
        calc,
    );
    let mut scope = SolverScope::new(director);
    scope.update_best_solution();

    let term = UnimprovedStepCountTermination::<TestSolution>::new(3);

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
            score: Some(SimpleScore::of(-5)),
        },
        SimpleScore::of(-5),
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
fn test_and_termination() {
    let mut scope = create_test_scope_with_score(SimpleScore::of(-10));

    // Both must be true: best score >= 0 AND step count >= 3
    let term = AndTermination::new((
        BestScoreTermination::new(SimpleScore::of(0)),
        StepCountTermination::new(3),
    ));

    // Neither condition met
    assert!(!term.is_terminated(&scope));

    // Only step count met
    scope.increment_step_count();
    scope.increment_step_count();
    scope.increment_step_count();
    assert!(!term.is_terminated(&scope));

    // Now set best score to meet first condition too
    scope.set_best_solution(
        TestSolution {
            score: Some(SimpleScore::of(0)),
        },
        SimpleScore::of(0),
    );
    assert!(term.is_terminated(&scope));
}

#[test]
fn test_or_termination() {
    let mut scope = create_test_scope_with_score(SimpleScore::of(-10));

    // Either: best score >= 0 OR step count >= 3
    let term = OrTermination::new((
        BestScoreTermination::new(SimpleScore::of(0)),
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
    let scope = create_test_scope();
    let term = UnimprovedTimeTermination::<TestSolution>::millis(10);

    // No score yet, should not terminate
    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_unimproved_time_termination_initial_score() {
    let scope = create_test_scope_with_score(SimpleScore::of(-10));
    let term = UnimprovedTimeTermination::<TestSolution>::millis(100);

    // First check records the score, should not terminate
    assert!(!term.is_terminated(&scope));
}

// Diminished returns termination tests

use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_diminished_not_terminated_during_grace_period() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(100), 0.0);

    let scope = create_scope_with_score(SimpleScore::of(-100));

    // During grace period, should not terminate even with no improvement
    assert!(!termination.is_terminated(&scope));
}

#[test]
fn test_diminished_terminates_with_zero_improvement() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(500), 0.1);

    let scope = create_scope_with_score(SimpleScore::of(-100));

    // First call starts tracking
    assert!(!termination.is_terminated(&scope));

    // Brief pause, well within 500ms grace period
    sleep(Duration::from_millis(50));
    assert!(!termination.is_terminated(&scope));

    // Wait past grace period
    sleep(Duration::from_millis(500));
    assert!(termination.is_terminated(&scope));
}

#[test]
fn test_diminished_not_terminated_with_sufficient_improvement() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(50), 10.0);

    let mut scope = create_scope_with_score(SimpleScore::of(-100));

    // Check once to start tracking
    assert!(!termination.is_terminated(&scope));

    sleep(Duration::from_millis(60));

    // Significant improvement: -100 -> 0 = +100 improvement over ~60ms
    // Rate = 100 / 0.060 = ~1667/s, well above 10/s threshold
    scope.set_best_solution(
        TestSolution {
            score: Some(SimpleScore::of(0)),
        },
        SimpleScore::of(0),
    );
    assert!(!termination.is_terminated(&scope));
}

#[test]
fn test_diminished_no_score_does_not_terminate() {
    let termination =
        DiminishedReturnsTermination::<TestSolution>::new(Duration::from_millis(10), 0.0);

    let scope = create_scope(); // No best score set

    sleep(Duration::from_millis(20));
    assert!(!termination.is_terminated(&scope));
}
