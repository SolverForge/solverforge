use std::time::{Duration, Instant};

use super::super::{SolverEvent, SolverLifecycleState, SolverManager, SolverTerminalReason};
use super::gates::{BlockingEvaluationGate, BlockingPoint};
use super::prompt_support::PromptControlSolution;

#[test]
fn retained_job_pause_settles_promptly_during_generation() {
    static MANAGER: SolverManager<PromptControlSolution> = SolverManager::new();

    let blocker = BlockingPoint::new();
    let solution = PromptControlSolution::generation_blocked(8_000, 512, blocker.clone(), None);
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    blocker.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");

    match receiver.blocking_recv().expect("pause requested event") {
        SolverEvent::PauseRequested { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    let resumed_at = Instant::now();
    blocker.release();

    match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        resumed_at.elapsed() < Duration::from_secs(1),
        "pause settlement after generation block took too long: {:?}",
        resumed_at.elapsed()
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
fn retained_job_cancel_settles_promptly_during_evaluation() {
    static MANAGER: SolverManager<PromptControlSolution> = SolverManager::new();

    let gate = BlockingEvaluationGate::new(96);
    let solution = PromptControlSolution::evaluation_blocked(8_000, gate.clone());
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    gate.wait_until_blocked();
    MANAGER.cancel(job_id).expect("cancel should be accepted");

    let released_at = Instant::now();
    gate.release();

    match receiver.blocking_recv().expect("cancelled event") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        released_at.elapsed() < Duration::from_secs(1),
        "cancel settlement after evaluation block took too long: {:?}",
        released_at.elapsed()
    );

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_time_limit_settles_promptly_during_generation() {
    static MANAGER: SolverManager<PromptControlSolution> = SolverManager::new();

    let blocker = BlockingPoint::new();
    let solution = PromptControlSolution::generation_blocked(
        8_000,
        512,
        blocker.clone(),
        Some(Duration::from_millis(20)),
    );
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match receiver.blocking_recv().expect("best solution event") {
        SolverEvent::BestSolution { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }

    blocker.wait_until_blocked();
    std::thread::sleep(Duration::from_millis(40));

    let released_at = Instant::now();
    blocker.release();

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::TerminatedByConfig)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert!(
        released_at.elapsed() < Duration::from_secs(1),
        "config termination after generation block took too long: {:?}",
        released_at.elapsed()
    );

    MANAGER.delete(job_id).expect("delete completed job");
}
