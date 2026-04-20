// Tests for scope types.

use std::any::TypeId;

use super::*;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use crate::test_utils::create_simple_nqueens_director;
use solverforge_scoring::ScoreDirector;
use solverforge_core::score::SoftScore;

#[test]
fn test_solver_scope_creation() {
    let director = create_simple_nqueens_director(2);
    let scope = SolverScope::new(director);

    assert!(scope.best_solution().is_none());
    assert!(scope.best_score().is_none());
    assert_eq!(scope.total_step_count(), 0);
}

#[test]
fn test_solver_scope_update_best() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);

    scope.update_best_solution();

    assert!(scope.best_solution().is_some());
    assert!(scope.best_score().is_some());
}

#[test]
fn test_solver_scope_step_count() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);

    assert_eq!(scope.increment_step_count(), 1);
    assert_eq!(scope.increment_step_count(), 2);
    assert_eq!(scope.total_step_count(), 2);
}

#[test]
fn test_phase_scope() {
    let director = create_simple_nqueens_director(2);
    let mut solver_scope = SolverScope::new(director);

    {
        let mut phase_scope = PhaseScope::new(&mut solver_scope, 0);
        assert_eq!(phase_scope.phase_index(), 0);
        assert_eq!(phase_scope.step_count(), 0);

        phase_scope.increment_step_count();
        assert_eq!(phase_scope.step_count(), 1);
    }

    assert_eq!(solver_scope.total_step_count(), 1);
}

#[test]
fn test_step_scope() {
    let director = create_simple_nqueens_director(2);
    let mut solver_scope = SolverScope::new(director);

    {
        let mut phase_scope = PhaseScope::new(&mut solver_scope, 0);

        {
            let mut step_scope = StepScope::new(&mut phase_scope);
            assert_eq!(step_scope.step_index(), 0);

            step_scope.set_step_score(SoftScore::of(-5));
            assert_eq!(step_scope.step_score(), Some(&SoftScore::of(-5)));

            step_scope.complete();
        }

        assert_eq!(phase_scope.step_count(), 1);
    }
}

#[derive(Clone, Debug)]
struct TieSolution {
    marker: usize,
    score: Option<SoftScore>,
}

impl PlanningSolution for TieSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn test_solver_scope_promotes_current_solution_on_score_tie() {
    let descriptor = SolutionDescriptor::new("TieSolution", TypeId::of::<TieSolution>());
    let director = ScoreDirector::simple(
        TieSolution {
            marker: 0,
            score: None,
        },
        descriptor,
        |_solution, _descriptor_index| 0,
    );
    let mut scope = SolverScope::new(director);

    scope.start_solving();
    scope.update_best_solution();
    assert_eq!(
        scope
            .best_solution()
            .expect("best solution should exist after update")
            .marker,
        0
    );

    scope.working_solution_mut().marker = 7;
    scope.promote_current_solution_on_score_tie();
    assert_eq!(
        scope
            .best_solution()
            .expect("tie promotion should publish the current solution")
            .marker,
        7
    );
}
