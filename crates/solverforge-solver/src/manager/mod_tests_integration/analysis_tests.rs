use solverforge_core::score::SoftScore;

use super::super::{SolverEvent, SolverLifecycleState, SolverManager, SolverTerminalReason};
use super::common::recv_event;
use super::lifecycle_solutions::LifecycleSolution;
use super::resume_support::{ConfigTerminatedSolution, FailureAfterSnapshotSolution};

#[test]
fn retained_job_analysis_is_snapshot_bound_across_live_states_and_completion() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let solution = LifecycleSolution::new(13);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match recv_event(&mut receiver, "best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let solving_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis while solving");
    assert_eq!(
        solving_analysis.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert_eq!(solving_analysis.snapshot_revision, 1);
    assert_eq!(solving_analysis.analysis.score, SoftScore::of(13));

    match recv_event(&mut receiver, "progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.pause(job_id).expect("pause should be accepted");

    match recv_event(&mut receiver, "pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let pause_requested_status = MANAGER
        .get_status(job_id)
        .expect("status while pause is requested");
    assert_eq!(
        pause_requested_status.lifecycle_state,
        SolverLifecycleState::PauseRequested
    );

    let pause_requested_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis while pause is requested");
    assert_eq!(
        pause_requested_analysis.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert_eq!(pause_requested_analysis.snapshot_revision, 1);
    assert_eq!(pause_requested_analysis.analysis.score, SoftScore::of(13));
    assert!(!pause_requested_analysis.lifecycle_state.is_terminal());

    gate.allow_next_step();

    match recv_event(&mut receiver, "paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let paused_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis while paused");
    assert_eq!(
        paused_analysis.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert_eq!(paused_analysis.snapshot_revision, 2);
    assert_eq!(paused_analysis.analysis.score, SoftScore::of(13));

    MANAGER.resume(job_id).expect("resume should be accepted");

    match recv_event(&mut receiver, "resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut receiver, "post-resume progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut receiver, "completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(3));
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let completed_analysis = MANAGER
        .analyze_snapshot(job_id, None)
        .expect("analysis after completion");
    assert_eq!(
        completed_analysis.lifecycle_state,
        SolverLifecycleState::Completed
    );
    assert_eq!(
        completed_analysis.terminal_reason,
        Some(SolverTerminalReason::Completed)
    );
    assert_eq!(completed_analysis.snapshot_revision, 3);
    assert_eq!(completed_analysis.analysis.score, SoftScore::of(13));

    MANAGER.delete(job_id).expect("delete completed job");
}

#[test]
fn retained_job_analysis_remains_available_after_cancel_failure_and_config_termination() {
    static CANCEL_MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();
    static FAILURE_MANAGER: SolverManager<FailureAfterSnapshotSolution> = SolverManager::new();
    static TERMINATED_MANAGER: SolverManager<ConfigTerminatedSolution> = SolverManager::new();

    let cancelled = LifecycleSolution::new(5);
    let cancel_gate = cancelled.gate.clone();
    let (cancelled_job_id, mut cancelled_receiver) = CANCEL_MANAGER
        .solve(cancelled)
        .expect("cancelled job should start");

    match recv_event(&mut cancelled_receiver, "cancelled job best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }
    match recv_event(&mut cancelled_receiver, "cancelled job progress event") {
        SolverEvent::Progress { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    CANCEL_MANAGER
        .pause(cancelled_job_id)
        .expect("pause should be accepted");
    match recv_event(
        &mut cancelled_receiver,
        "cancelled job pause requested event",
    ) {
        SolverEvent::PauseRequested { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    cancel_gate.allow_next_step();

    match recv_event(&mut cancelled_receiver, "cancelled job paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    CANCEL_MANAGER
        .cancel(cancelled_job_id)
        .expect("cancel should be accepted");
    match recv_event(&mut cancelled_receiver, "cancelled job cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let cancelled_analysis = CANCEL_MANAGER
        .analyze_snapshot(cancelled_job_id, None)
        .expect("analysis after cancellation");
    assert_eq!(
        cancelled_analysis.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert_eq!(cancelled_analysis.snapshot_revision, 2);
    assert_eq!(cancelled_analysis.analysis.score, SoftScore::of(5));

    let (failed_job_id, mut failed_receiver) = FAILURE_MANAGER
        .solve(FailureAfterSnapshotSolution::new(17))
        .expect("failed job should start");

    match recv_event(&mut failed_receiver, "failed job best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut failed_receiver, "failed job failed event") {
        SolverEvent::Failed { metadata, error } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Failed);
            assert!(error.contains("expected retained lifecycle failure"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let failed_analysis = FAILURE_MANAGER
        .analyze_snapshot(failed_job_id, None)
        .expect("analysis after failure");
    assert_eq!(
        failed_analysis.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert_eq!(failed_analysis.snapshot_revision, 1);
    assert_eq!(failed_analysis.analysis.score, SoftScore::of(17));

    let (terminated_job_id, mut terminated_receiver) = TERMINATED_MANAGER
        .solve(ConfigTerminatedSolution::new(23))
        .expect("configured-termination job should start");

    match recv_event(
        &mut terminated_receiver,
        "configured-termination best solution event",
    ) {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(
        &mut terminated_receiver,
        "configured-termination completed event",
    ) {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::TerminatedByConfig)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let terminated_analysis = TERMINATED_MANAGER
        .analyze_snapshot(terminated_job_id, None)
        .expect("analysis after configured termination");
    assert_eq!(
        terminated_analysis.lifecycle_state,
        SolverLifecycleState::Completed
    );
    assert_eq!(
        terminated_analysis.terminal_reason,
        Some(SolverTerminalReason::TerminatedByConfig)
    );
    assert_eq!(terminated_analysis.snapshot_revision, 2);
    assert_eq!(terminated_analysis.analysis.score, SoftScore::of(23));

    CANCEL_MANAGER
        .delete(cancelled_job_id)
        .expect("delete cancelled job");
    FAILURE_MANAGER
        .delete(failed_job_id)
        .expect("delete failed job");
    TERMINATED_MANAGER
        .delete(terminated_job_id)
        .expect("delete configured-termination job");
}
