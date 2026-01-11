//! Integration tests for termination conditions.

use super::*;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

#[derive(Clone, Debug)]
struct TestSolution {
    score: Option<SimpleScore>,
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

type TestDirector = SimpleScoreDirector<TestSolution, fn(&TestSolution) -> SimpleScore>;

fn calc(_: &TestSolution) -> SimpleScore {
    SimpleScore::of(0)
}

fn create_scope() -> SolverScope<'static, TestSolution, TestDirector> {
    let desc = SolutionDescriptor::new("Test", TypeId::of::<TestSolution>());
    let director = SimpleScoreDirector::with_calculator(
        TestSolution { score: None },
        desc,
        calc as fn(&TestSolution) -> SimpleScore,
    );
    SolverScope::new(director)
}

fn create_scope_with_score(
    score: SimpleScore,
) -> SolverScope<'static, TestSolution, SimpleScoreDirector<TestSolution, impl Fn(&TestSolution) -> SimpleScore>>
{
    let desc = SolutionDescriptor::new("Test", TypeId::of::<TestSolution>());
    let score_clone = score;
    let director = SimpleScoreDirector::with_calculator(
        TestSolution { score: Some(score) },
        desc,
        move |_| score_clone,
    );
    let mut scope = SolverScope::new(director);
    scope.update_best_solution();
    scope
}

#[test]
fn test_step_count_termination() {
    let mut scope = create_scope();
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
    let scope = create_scope_with_score(SimpleScore::of(-5));
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_reached() {
    let scope = create_scope_with_score(SimpleScore::of(0));
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_exceeded() {
    let scope = create_scope_with_score(SimpleScore::of(5));
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_termination_no_score() {
    let scope = create_scope();
    let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_feasible_termination() {
    let scope = create_scope_with_score(SimpleScore::of(0));
    let term = BestScoreFeasibleTermination::<TestSolution, _>::score_at_least_zero();

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_best_score_feasible_termination_not_feasible() {
    let scope = create_scope_with_score(SimpleScore::of(-5));
    let term = BestScoreFeasibleTermination::<TestSolution, _>::score_at_least_zero();

    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_best_score_feasible_termination_custom() {
    let scope = create_scope_with_score(SimpleScore::of(-3));
    // Custom feasibility: score >= -5 is considered feasible
    let term = BestScoreFeasibleTermination::<TestSolution, _>::new(|score: &SimpleScore| {
        *score >= SimpleScore::of(-5)
    });

    assert!(term.is_terminated(&scope));
}

#[test]
fn test_unimproved_step_count_termination() {
    let mut scope = create_scope_with_score(SimpleScore::of(-10));
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
    let mut scope = create_scope_with_score(SimpleScore::of(-10));

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
    let mut scope = create_scope_with_score(SimpleScore::of(-10));

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
    let scope = create_scope();
    let term = UnimprovedTimeTermination::<TestSolution>::millis(10);

    // No score yet, should not terminate
    assert!(!term.is_terminated(&scope));
}

#[test]
fn test_unimproved_time_termination_initial_score() {
    let scope = create_scope_with_score(SimpleScore::of(-10));
    let term = UnimprovedTimeTermination::<TestSolution>::millis(100);

    // First check records the score, should not terminate
    assert!(!term.is_terminated(&scope));
}
