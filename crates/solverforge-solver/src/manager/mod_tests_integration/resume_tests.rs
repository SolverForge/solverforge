use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;

use super::super::{SolverEvent, SolverLifecycleState, SolverManager};
use super::common::recv_event;
use super::resume_support::{DeterministicResumeSolution, TracePairingResumeSolution};

#[test]
fn retained_job_exact_resume_matches_uninterrupted_execution_after_boundary() {
    static MANAGER: SolverManager<DeterministicResumeSolution> = SolverManager::new();

    let uninterrupted = DeterministicResumeSolution::new();
    let uninterrupted_gate = uninterrupted.gate.clone();
    let (uninterrupted_job_id, mut uninterrupted_receiver) = MANAGER
        .solve(uninterrupted)
        .expect("uninterrupted job should start");

    match recv_event(
        &mut uninterrupted_receiver,
        "uninterrupted best solution event",
    ) {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(solution.value, 10);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut uninterrupted_receiver, "uninterrupted progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(metadata.current_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.telemetry.step_count, 1);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    uninterrupted_gate.allow_next_step();

    let uninterrupted_boundary_snapshot = match recv_event(
        &mut uninterrupted_receiver,
        "uninterrupted boundary snapshot event",
    ) {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.current_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.telemetry.step_count, 2);
            assert_eq!(solution.value, 12);
            solution
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let uninterrupted_post_boundary = match recv_event(
        &mut uninterrupted_receiver,
        "uninterrupted post-boundary progress",
    ) {
        SolverEvent::Progress { metadata } => (
            metadata.snapshot_revision,
            metadata.current_score,
            metadata.best_score,
            metadata.telemetry.step_count,
        ),
        other => panic!("unexpected event: {other:?}"),
    };

    let uninterrupted_completed =
        match recv_event(&mut uninterrupted_receiver, "uninterrupted completed event") {
            SolverEvent::Completed { metadata, solution } => (
                metadata.snapshot_revision,
                metadata.current_score,
                metadata.best_score,
                metadata.terminal_reason,
                metadata.telemetry.step_count,
                solution.value,
            ),
            other => panic!("unexpected event: {other:?}"),
        };

    let resumed = DeterministicResumeSolution::new();
    let resumed_gate = resumed.gate.clone();
    let (resumed_job_id, mut resumed_receiver) =
        MANAGER.solve(resumed).expect("resumed job should start");

    match recv_event(&mut resumed_receiver, "resumed best solution event") {
        SolverEvent::BestSolution { metadata, solution } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(solution.value, 10);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut resumed_receiver, "resumed progress event") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert_eq!(metadata.current_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(10)));
            assert_eq!(metadata.telemetry.step_count, 1);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER
        .pause(resumed_job_id)
        .expect("pause should be accepted");

    match recv_event(&mut resumed_receiver, "pause requested event") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    resumed_gate.allow_next_step();

    match recv_event(&mut resumed_receiver, "paused boundary snapshot event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.current_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.best_score, Some(SoftScore::of(12)));
            assert_eq!(metadata.telemetry.step_count, 2);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let resumed_boundary_snapshot = MANAGER
        .get_snapshot(resumed_job_id, Some(2))
        .expect("paused boundary snapshot");
    assert_eq!(resumed_boundary_snapshot.snapshot_revision, 2);
    assert_eq!(
        resumed_boundary_snapshot.current_score,
        Some(SoftScore::of(12))
    );
    assert_eq!(
        resumed_boundary_snapshot.best_score,
        Some(SoftScore::of(12))
    );
    assert_eq!(resumed_boundary_snapshot.telemetry.step_count, 2);
    assert_eq!(resumed_boundary_snapshot.solution.value, 12);
    assert_eq!(
        resumed_boundary_snapshot.solution.score(),
        Some(SoftScore::of(12))
    );

    assert_eq!(
        uninterrupted_boundary_snapshot.value,
        resumed_boundary_snapshot.solution.value
    );
    assert_eq!(
        uninterrupted_boundary_snapshot.score(),
        resumed_boundary_snapshot.solution.score()
    );

    MANAGER
        .resume(resumed_job_id)
        .expect("resume should be accepted");

    match recv_event(&mut resumed_receiver, "resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert_eq!(metadata.best_score, Some(SoftScore::of(12)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let resumed_post_boundary =
        match recv_event(&mut resumed_receiver, "resumed post-boundary progress") {
            SolverEvent::Progress { metadata } => (
                metadata.snapshot_revision,
                metadata.current_score,
                metadata.best_score,
                metadata.telemetry.step_count,
            ),
            other => panic!("unexpected event: {other:?}"),
        };

    let resumed_completed = match recv_event(&mut resumed_receiver, "resumed completed event") {
        SolverEvent::Completed { metadata, solution } => (
            metadata.snapshot_revision,
            metadata.current_score,
            metadata.best_score,
            metadata.terminal_reason,
            metadata.telemetry.step_count,
            solution.value,
        ),
        other => panic!("unexpected event: {other:?}"),
    };

    assert_eq!(resumed_post_boundary, uninterrupted_post_boundary);
    assert_eq!(resumed_completed, uninterrupted_completed);

    let uninterrupted_final_snapshot = MANAGER
        .get_snapshot(uninterrupted_job_id, None)
        .expect("uninterrupted final snapshot");
    let resumed_final_snapshot = MANAGER
        .get_snapshot(resumed_job_id, None)
        .expect("resumed final snapshot");

    assert_eq!(
        uninterrupted_final_snapshot.snapshot_revision,
        resumed_final_snapshot.snapshot_revision
    );
    assert_eq!(
        uninterrupted_final_snapshot.current_score,
        resumed_final_snapshot.current_score
    );
    assert_eq!(
        uninterrupted_final_snapshot.best_score,
        resumed_final_snapshot.best_score
    );
    assert_eq!(
        uninterrupted_final_snapshot.solution.value,
        resumed_final_snapshot.solution.value
    );

    MANAGER
        .delete(uninterrupted_job_id)
        .expect("delete uninterrupted job");
    MANAGER.delete(resumed_job_id).expect("delete resumed job");
}

#[test]
fn retained_candidate_trace_detail_is_paired_with_its_exact_snapshot_publication() {
    static MANAGER: SolverManager<TracePairingResumeSolution> = SolverManager::new();

    let solution = TracePairingResumeSolution::new();
    let pause_boundary = solution.pause_boundary.clone();
    let resumed_boundary = solution.resumed_boundary.clone();
    let progress_boundary = solution.progress_boundary.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("trace pairing job starts");

    match recv_event(&mut receiver, "initial trace pairing snapshot") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(metadata.event_sequence, 1);
            assert_eq!(metadata.snapshot_revision, Some(1));
            assert!(metadata.telemetry.candidate_trace.is_none());
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.pause(job_id).expect("pause accepted");
    match recv_event(&mut receiver, "trace pairing pause requested") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(metadata.event_sequence, 2);
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
            assert!(metadata.telemetry.candidate_trace.is_none());
        }
        other => panic!("unexpected event: {other:?}"),
    }

    pause_boundary.allow_next_step();
    let paused = match recv_event(&mut receiver, "trace pairing paused") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.event_sequence, 3);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            assert_eq!(metadata.snapshot_revision, Some(2));
            assert!(metadata.telemetry.candidate_trace.is_none());
            metadata
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let paused_detail = MANAGER
        .get_telemetry_detail(job_id)
        .expect("paused trace detail");
    assert_eq!(paused_detail.status.event_sequence, paused.event_sequence);
    assert_eq!(
        paused_detail.status.latest_snapshot_revision,
        paused.snapshot_revision
    );
    assert_eq!(
        paused_detail.status.lifecycle_state,
        SolverLifecycleState::Paused
    );
    assert!(paused_detail.status.telemetry.candidate_trace.is_none());
    assert_eq!(
        paused_detail
            .candidate_trace
            .as_ref()
            .expect("pause publication carries the detached trace")
            .header
            .configured_input,
        "retained-trace-pairing-pause"
    );

    MANAGER.resume(job_id).expect("resume accepted");
    let resumed = match recv_event(&mut receiver, "trace pairing resumed") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.event_sequence, 4);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.snapshot_revision, paused.snapshot_revision);
            assert!(metadata.telemetry.candidate_trace.is_none());
            metadata
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let resumed_detail = MANAGER
        .get_telemetry_detail(job_id)
        .expect("resumed compact detail");
    assert_eq!(resumed_detail.status.event_sequence, resumed.event_sequence);
    assert_eq!(
        resumed_detail.status.latest_snapshot_revision,
        resumed.snapshot_revision
    );
    assert_eq!(
        resumed_detail.status.lifecycle_state,
        SolverLifecycleState::Solving
    );
    assert!(resumed_detail.candidate_trace.is_none());

    resumed_boundary.allow_next_step();
    let progress = match recv_event(&mut receiver, "trace pairing progress") {
        SolverEvent::Progress { metadata } => {
            assert_eq!(metadata.event_sequence, 5);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
            assert_eq!(metadata.snapshot_revision, paused.snapshot_revision);
            assert!(metadata.telemetry.candidate_trace.is_none());
            metadata
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let progress_detail = MANAGER
        .get_telemetry_detail(job_id)
        .expect("progress compact detail");
    assert_eq!(
        progress_detail.status.event_sequence,
        progress.event_sequence
    );
    assert_eq!(
        progress_detail.status.latest_snapshot_revision,
        progress.snapshot_revision
    );
    assert!(progress_detail.candidate_trace.is_none());

    progress_boundary.allow_next_step();
    let completed = match recv_event(&mut receiver, "trace pairing completed") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.event_sequence, 6);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(metadata.snapshot_revision, Some(3));
            assert!(metadata.telemetry.candidate_trace.is_none());
            metadata
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let completed_detail = MANAGER
        .get_telemetry_detail(job_id)
        .expect("completed compact detail");
    assert_eq!(
        completed_detail.status.event_sequence,
        completed.event_sequence
    );
    assert_eq!(
        completed_detail.status.latest_snapshot_revision,
        completed.snapshot_revision
    );
    assert_eq!(
        completed_detail.status.lifecycle_state,
        SolverLifecycleState::Completed
    );
    assert_eq!(completed_detail.status.best_score, completed.best_score);
    assert_eq!(
        completed_detail.status.current_score,
        completed.current_score
    );
    assert_eq!(
        completed_detail
            .candidate_trace
            .as_ref()
            .expect("terminal publication carries its detached trace")
            .header
            .configured_input,
        "retained-trace-pairing-terminal"
    );

    let snapshot_revision = completed_detail
        .status
        .latest_snapshot_revision
        .expect("terminal trace detail includes the exact snapshot revision");
    let terminal_snapshot = MANAGER
        .get_snapshot(job_id, Some(snapshot_revision))
        .expect("terminal trace detail revision resolves to a retained snapshot");
    assert_eq!(terminal_snapshot.snapshot_revision, snapshot_revision);
    assert_eq!(
        terminal_snapshot.lifecycle_state,
        SolverLifecycleState::Completed
    );
    assert_eq!(
        terminal_snapshot.current_score,
        completed_detail.status.current_score
    );
    assert_eq!(
        terminal_snapshot.best_score,
        completed_detail.status.best_score
    );
    assert_eq!(
        terminal_snapshot.telemetry, completed_detail.status.telemetry,
        "the trace-bearing status and referenced snapshot share one publication"
    );

    MANAGER
        .delete(job_id)
        .expect("delete completed trace pairing job");
}
