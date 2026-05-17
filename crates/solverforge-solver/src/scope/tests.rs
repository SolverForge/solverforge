// Tests for scope types.

use std::any::TypeId;

use super::*;
use crate::manager::SolverTerminalReason;
use crate::phase::construction::{ConstructionListElementId, ConstructionSlotId};
use crate::test_utils::create_simple_nqueens_director;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_scoring::ScoreDirector;

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
fn inphase_best_score_limit_requests_search_config_termination() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);
    scope.start_solving();
    scope.install_inphase_best_score_limit(SoftScore::of(0));
    let solution = scope.score_director().clone_working_solution();
    scope.set_best_solution(solution, SoftScore::of(0));

    assert_eq!(
        scope.pending_control(),
        PendingControl::ConfigTerminationRequested
    );
    assert!(scope.should_terminate());
    assert_eq!(
        scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn inphase_best_score_limit_requests_construction_config_termination() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);
    scope.start_solving();
    scope.install_inphase_best_score_limit(SoftScore::of(0));
    let solution = scope.score_director().clone_working_solution();
    scope.set_best_solution(solution, SoftScore::of(0));

    assert!(scope.should_terminate_construction());
    assert_eq!(
        scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn mandatory_construction_control_ignores_config_termination() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);
    scope.start_solving();
    scope.set_time_limit(std::time::Duration::ZERO);
    scope.install_inphase_best_score_limit(SoftScore::of(0));
    let solution = scope.score_director().clone_working_solution();
    scope.set_best_solution(solution, SoftScore::of(0));

    assert_eq!(
        scope.pending_control(),
        PendingControl::ConfigTerminationRequested
    );
    assert!(scope.work_should_stop());
    assert_eq!(
        scope.mandatory_construction_pending_control(),
        PendingControl::Continue
    );
    assert!(!scope.mandatory_construction_work_should_stop());
    assert!(!scope.should_interrupt_mandatory_construction());
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

    scope.mutate(|score_director| {
        score_director.working_solution_mut().marker = 7;
    });
    scope.calculate_score();
    scope.promote_current_solution_on_score_tie();
    assert_eq!(
        scope
            .best_solution()
            .expect("tie promotion should publish the current solution")
            .marker,
        7
    );
}

#[test]
fn test_solver_scope_mutate_advances_revision_once() {
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
    let initial_revision = scope.solution_revision();
    scope.set_current_score(SoftScore::of(0));

    scope.mutate(|score_director| {
        score_director.working_solution_mut().marker = 5;
    });

    assert_eq!(scope.solution_revision(), initial_revision + 1);
    assert!(scope.current_score().is_none());
    assert_eq!(scope.working_solution().marker, 5);
}

#[test]
fn test_replace_working_solution_reinitializes_revision_and_frontier() {
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

    let slot_id = ConstructionSlotId::new(0, 0);
    let element_id = ConstructionListElementId::new(0, 0);

    scope.mark_scalar_slot_completed(slot_id);
    scope.mark_list_element_completed(element_id);
    scope.mutate(|score_director| {
        score_director.working_solution_mut().marker = 3;
    });
    assert!(scope.solution_revision() > 1);

    let score = scope.replace_working_solution_and_reinitialize(TieSolution {
        marker: 9,
        score: None,
    });

    assert_eq!(score, SoftScore::of(0));
    assert_eq!(scope.solution_revision(), 1);
    assert!(!scope.is_scalar_slot_completed(slot_id));
    assert!(!scope.is_list_element_completed(element_id));
    assert_eq!(scope.working_solution().marker, 9);
}
