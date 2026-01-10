//! Tests for SolverManager and related types.

use super::*;

use solverforge_core::domain::SolutionDescriptor;
use solverforge_core::score::SimpleScore;
use solverforge_scoring::SimpleScoreDirector;
use std::any::TypeId;

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

impl<S: PlanningSolution, D: solverforge_scoring::ScoreDirector<S>> crate::phase::Phase<S, D>
    for NoOpPhase
{
    fn solve(&mut self, solver_scope: &mut crate::scope::SolverScope<S, D>) {
        solver_scope.update_best_solution();
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOpPhase"
    }
}

#[test]
fn test_solver_manager_builder_creation() {
    let _builder = SolverManager::<TestSolution, TestDirector>::builder();
}

#[test]
fn test_solver_manager_builder_builds_successfully() {
    let _manager = SolverManager::<TestSolution, TestDirector>::builder()
        .build()
        .expect("Failed to build SolverManager");
}

#[test]
fn test_cloneable_phase_factory() {
    let factory = CloneablePhaseFactory::new(NoOpPhase);
    let phase: Box<dyn crate::phase::Phase<TestSolution, TestDirector>> = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_closure_phase_factory() {
    let factory = ClosurePhaseFactory::<TestSolution, _>::new(|| {
        Box::new(NoOpPhase) as Box<dyn crate::phase::Phase<TestSolution, TestDirector>>
    });

    let phase: Box<dyn crate::phase::Phase<TestSolution, TestDirector>> = factory.create_phase();
    assert_eq!(phase.phase_type_name(), "NoOpPhase");
}

#[test]
fn test_create_solver_returns_valid_solver() {
    let manager = SolverManager::<TestSolution, TestDirector>::builder()
        .build()
        .expect("Failed to build SolverManager");

    let solver = manager.create_solver();
    assert!(!solver.is_solving());
}

#[test]
fn test_create_solver_with_termination() {
    let termination_factory: Box<
        dyn Fn() -> Box<dyn crate::termination::Termination<TestSolution, TestDirector>>
            + Send
            + Sync,
    > = Box::new(|| Box::new(StepCountTermination::new(50)));

    let manager = SolverManager::<TestSolution, TestDirector>::new(vec![], Some(termination_factory));

    let solver = manager.create_solver();
    assert!(!solver.is_solving());
}
