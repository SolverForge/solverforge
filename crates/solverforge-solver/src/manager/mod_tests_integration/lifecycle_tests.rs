use std::time::Duration;

use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;

use super::super::{
    SolverEvent, SolverLifecycleState, SolverManager, SolverManagerError, SolverTerminalReason,
};
use super::lifecycle_solutions::{
    DeleteReservationSolution, LifecycleSolution, PauseOrderingSolution,
    PauseRequestedProgressSolution, TrivialLifecycleSolution,
};

#[test]
fn retained_job_pause_resume_completion_flow() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let solution = LifecycleSolution::new(7);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.event_sequence, 1);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.event_sequence, 2);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(metadata.event_sequence, 3);
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    gate.allow_next_step();

    let paused_telemetry = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.event_sequence, 4);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.telemetry.step_count, 1);
            assert_eq!(metadata.telemetry.moves_generated, 1);
            assert_eq!(metadata.telemetry.moves_evaluated, 1);
            assert_eq!(metadata.telemetry.moves_accepted, 1);
            assert_eq!(metadata.telemetry.score_calculations, 1);
            assert_eq!(metadata.telemetry.generation_time, Duration::ZERO);
            assert_eq!(metadata.telemetry.evaluation_time, Duration::ZERO);
            metadata.telemetry
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let status = MANAGER.get_status(job_id).expect("status while paused");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Paused);
    assert!(status.checkpoint_available);
    assert_eq!(status.event_sequence, 4);
    assert_eq!(status.latest_snapshot_revision, Some(2));
    assert_eq!(status.telemetry, paused_telemetry);

    let paused_snapshot = MANAGER.get_snapshot(job_id, None).expect("paused snapshot");
    assert_eq!(
        paused_snapshot.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert_eq!(paused_snapshot.snapshot_revision, 2);
    assert_eq!(paused_snapshot.telemetry, paused_telemetry);

    let analysis = MANAGER
        .analyze_snapshot(job_id, Some(paused_snapshot.snapshot_revision))
        .expect("analysis for paused snapshot");
    assert_eq!(
        analysis.snapshot_revision,
        paused_snapshot.snapshot_revision
    );
    assert_eq!(analysis.lifecycle_state, SolverLifecycleState::Paused);
    assert_eq!(analysis.analysis.score, SoftScore::of(7));

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.event_sequence, 5);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("progress after resume") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.event_sequence, 6);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.telemetry.step_count, 2);
            assert_eq!(metadata.telemetry.moves_generated, 2);
            assert_eq!(metadata.telemetry.moves_evaluated, 2);
            assert_eq!(metadata.telemetry.moves_accepted, 2);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let completed_telemetry = match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.event_sequence, 7);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Completed)
            );
            assert_eq!(metadata.telemetry.step_count, 2);
            assert_eq!(metadata.telemetry.moves_generated, 2);
            assert_eq!(metadata.telemetry.moves_evaluated, 2);
            assert_eq!(metadata.telemetry.moves_accepted, 2);
            assert_eq!(metadata.telemetry.score_calculations, 1);
            metadata.telemetry
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let status = MANAGER.get_status(job_id).expect("status after completion");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Completed);
    assert_eq!(
        status.terminal_reason,
        Some(SolverTerminalReason::Completed)
    );
    assert_eq!(status.event_sequence, 7);
    assert!(!status.checkpoint_available);
    assert_eq!(status.telemetry, completed_telemetry);

    let completed_snapshot = MANAGER
        .get_snapshot(job_id, None)
        .expect("completed snapshot");
    assert_eq!(completed_snapshot.telemetry, completed_telemetry);
    assert_eq!(completed_snapshot.solution.score(), Some(SoftScore::of(7)));

    MANAGER.delete(job_id).expect("delete terminal job");
    assert!(matches!(
        MANAGER.get_status(job_id),
        Err(SolverManagerError::JobNotFound { .. })
    ));
}

#[test]
fn retained_job_invalid_transitions_cancel_and_delete() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let solution = LifecycleSolution::new(3);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    assert!(matches!(
        MANAGER.resume(job_id),
        Err(SolverManagerError::InvalidStateTransition { action, .. }) if action == "resume"
    ));

    assert!(matches!(
        MANAGER.delete(job_id),
        Err(SolverManagerError::InvalidStateTransition { action, .. }) if action == "delete"
    ));

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }
    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    gate.allow_next_step();

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Cancelled)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let status = MANAGER.get_status(job_id).expect("status after cancel");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Cancelled);
    assert_eq!(
        status.terminal_reason,
        Some(SolverTerminalReason::Cancelled)
    );

    MANAGER.delete(job_id).expect("delete cancelled job");
    assert!(matches!(
        MANAGER.get_status(job_id),
        Err(SolverManagerError::JobNotFound { .. })
    ));
}

#[test]
fn retained_job_progress_reflects_pause_requested_state() {
    static MANAGER: SolverManager<PauseRequestedProgressSolution> = SolverManager::new();

    let solution = PauseRequestedProgressSolution::new(11);
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    gate.allow_next_step();

    match receiver.blocking_recv().expect("progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_pause_requested_event_precedes_worker_pause_events() {
    static MANAGER: SolverManager<PauseOrderingSolution> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER
        .solve(PauseOrderingSolution::new(17))
        .expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(metadata.event_sequence, 2);
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let mut saw_pause_requested_progress = false;
    loop {
        match receiver
            .blocking_recv()
            .expect("pause lifecycle event after request")
        {
            SolverEvent::Progress { metadata } => {
                saw_pause_requested_progress = true;
                assert_eq!(
                    metadata.lifecycle_state,
                    SolverLifecycleState::PauseRequested
                );
                assert!(metadata.event_sequence > 2);
            }
            SolverEvent::Paused { metadata } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
                break;
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    assert!(
        saw_pause_requested_progress,
        "worker should emit pause-requested progress before settling the snapshot"
    );

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_delete_keeps_slot_reserved_until_worker_exit() {
    static MANAGER: SolverManager<DeleteReservationSolution> = SolverManager::new();

    let solution = DeleteReservationSolution::new();
    let release_return = solution.release_return.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete completed job");
    assert!(matches!(
        MANAGER.get_status(job_id),
        Err(SolverManagerError::JobNotFound { .. })
    ));
    assert_eq!(MANAGER.active_job_count(), 0);
    assert!(!MANAGER.slot_is_free_for_test(job_id));

    release_return.allow_next_step();

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while std::time::Instant::now() < deadline {
        if MANAGER.slot_is_free_for_test(job_id) {
            return;
        }
        std::thread::yield_now();
    }

    panic!("slot {job_id} was not released after the worker exited");
}

#[test]
fn trivial_job_cancelled_while_paused_reports_cancelled() {
    static MANAGER: SolverManager<TrivialLifecycleSolution> = SolverManager::new();

    let solution = TrivialLifecycleSolution::new();
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    gate.allow_next_step();

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Cancelled)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete cancelled job");
}
