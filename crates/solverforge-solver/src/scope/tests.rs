//! Tests for scope types.

use super::*;
use crate::test_utils::create_simple_nqueens_director;
use solverforge_core::score::SimpleScore;

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

            step_scope.set_step_score(SimpleScore::of(-5));
            assert_eq!(step_scope.step_score(), Some(&SimpleScore::of(-5)));

            step_scope.complete();
        }

        assert_eq!(phase_scope.step_count(), 1);
    }
}
