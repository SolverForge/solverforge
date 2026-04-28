use solverforge_core::score::SoftScore;
use solverforge_core::PlanningSolution;

use super::super::{SolverEvent, SolverLifecycleState, SolverManager};
use super::common::recv_event;
use super::resume_support::DeterministicResumeSolution;

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
