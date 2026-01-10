//! Tests for SolverManager and related types.

use super::*;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::solver::Solver;
use crate::termination::StepCountTermination;

#[derive(Clone, Debug)]
struct TestSolution {
    value: i64,
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

#[derive(Debug, Clone)]
struct NoOpPhase;

impl<S: PlanningSolution, D: ScoreDirector<S>> Phase<S, D> for NoOpPhase {
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}

#[test]
fn test_cloneable_phase_factory() {
    let factory = CloneablePhaseFactory::new(NoOpPhase);
    let phase: NoOpPhase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_closure_phase_factory() {
    let factory = ClosurePhaseFactory::<NoOpPhase, _>::new(|| NoOpPhase);
    let phase: NoOpPhase = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_solver_with_phase() {
    let solver: Solver<TestSolution, TestDirector, NoOpPhase, ()> = Solver::with_phase(NoOpPhase);
    assert!(!solver.is_solving());
}

#[test]
fn test_solver_with_termination() {
    let solver: Solver<TestSolution, TestDirector, NoOpPhase, StepCountTermination> =
        Solver::new(NoOpPhase, Some(StepCountTermination::new(50)));
    assert!(!solver.is_solving());
}

#[test]
fn test_solver_builder() {
    let solver = SolverBuilder::<TestSolution, TestDirector, _>::new(NoOpPhase)
        .with_step_limit(100)
        .build();
    assert!(!solver.is_solving());
}
