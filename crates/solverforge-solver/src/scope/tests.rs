// Tests for scope types.

use std::any::TypeId;

use super::*;
use crate::manager::SolverTerminalReason;
use crate::phase::construction::{ConstructionListElementId, ConstructionSlotId};
use crate::stats::{CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan};
use crate::test_utils::{create_minimal_director, create_simple_nqueens_director, TestSolution};
use solverforge_config::TerminationConfig;
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
fn scoped_phase_termination_is_relative_complete_and_restored() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);
    scope.start_solving();
    let solution = scope.score_director().clone_working_solution();
    scope.set_best_solution(solution, SoftScore::of(0));

    for termination in [
        TerminationConfig {
            best_score_limit: Some("0".to_string()),
            ..TerminationConfig::default()
        },
        TerminationConfig {
            step_count_limit: Some(0),
            ..TerminationConfig::default()
        },
        TerminationConfig {
            unimproved_step_count_limit: Some(0),
            ..TerminationConfig::default()
        },
        TerminationConfig {
            unimproved_seconds_spent_limit: Some(0),
            ..TerminationConfig::default()
        },
    ] {
        scope.with_phase_termination(Some(&termination), |scope| {
            assert!(scope.config_control_polling_required());
            assert_eq!(
                scope.pending_control(),
                PendingControl::ConfigTerminationRequested
            );
            assert!(scope.should_terminate());
        });
        assert_eq!(scope.pending_control(), PendingControl::Continue);
    }

    let time_limited = TerminationConfig {
        seconds_spent_limit: Some(1),
        ..TerminationConfig::default()
    };
    scope.with_phase_termination(Some(&time_limited), |scope| {
        assert!(scope.config_control_polling_required());
        assert_eq!(scope.pending_control(), PendingControl::Continue);
    });
    assert_eq!(scope.terminal_reason(), SolverTerminalReason::Completed);
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
fn phase_progress_elapsed_uses_the_pause_aware_solver_clock() {
    let publications = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = std::sync::Arc::clone(&publications);
    let callback = move |progress: SolverProgressRef<'_, TestSolution>| {
        captured.lock().unwrap().push(progress.telemetry);
    };
    let mut solver_scope =
        SolverScope::new_with_callback(create_minimal_director(), callback, None, None);
    solver_scope.start_solving();

    let mut phase_scope = PhaseScope::with_phase_type(&mut solver_scope, 0, "PauseAwarePhase");
    phase_scope.solver_scope_mut().pause_timers();
    let frozen_elapsed = phase_scope.elapsed();
    let raw_before = phase_scope.stats().elapsed();
    for value in 0..100_000 {
        std::hint::black_box(value);
    }
    let raw_after = phase_scope.stats().elapsed();

    assert_eq!(phase_scope.elapsed(), frozen_elapsed);
    assert!(raw_after > raw_before);
    phase_scope.report_progress();
    drop(phase_scope);

    let telemetry = publications
        .lock()
        .unwrap()
        .pop()
        .expect("phase progress is published");
    let phase = telemetry.phase.expect("phase telemetry is attached");
    assert_eq!(phase.phase_type, "PauseAwarePhase");
    assert_eq!(phase.elapsed, frozen_elapsed);
    assert!(phase.elapsed < raw_after);
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

#[test]
fn solver_scope_forwards_candidate_trace_plan_finalization() {
    let director = create_simple_nqueens_director(2);
    let mut scope = SolverScope::new(director);
    scope.enable_candidate_trace(
        CandidateTraceHeader::new(
            "[candidate_trace]\nmax_entries = 1\n".to_string(),
            CandidateTraceExecutionPolicy::known(
                "test.execution_policy",
                std::iter::empty::<(String, String)>(),
            ),
            CandidateTracePhasePlan::opaque("test.pending"),
            None,
        ),
        1,
    );

    let terminal_plan =
        CandidateTracePhasePlan::known("test.terminal", [("outcome", "completed")], Vec::new());
    scope.finalize_candidate_trace_resolved_phase_plan(terminal_plan.clone());

    let trace = scope
        .stats()
        .snapshot()
        .candidate_trace
        .expect("enabled candidate trace");
    assert_eq!(trace.header.resolved_phase_plan, terminal_plan);
    assert!(trace.header.resolved_phase_plan_complete);
}
