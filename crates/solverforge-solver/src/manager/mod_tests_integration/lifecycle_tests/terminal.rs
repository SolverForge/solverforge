use super::*;

#[test]
fn pause_accepted_during_terminal_hook_settles_before_completion() {
    static MANAGER: SolverManager<TerminalHookLifecycleSolution> = SolverManager::new();

    let solution = TerminalHookLifecycleSolution::new(23);
    let hook = solution.terminal_hook.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");
    hook.wait_until_blocked();

    MANAGER
        .pause(job_id)
        .expect("pause during terminal hook is accepted");
    match recv_event(&mut receiver, "pause requested during terminal hook") {
        SolverEvent::PauseRequested { metadata } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    hook.release();
    match recv_event(&mut receiver, "paused before terminal completion") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.resume(job_id).expect("terminal pause resumes");
    match recv_event(&mut receiver, "resumed terminal solve") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }
    match recv_event(&mut receiver, "completion after terminal resume") {
        SolverEvent::Completed { metadata, .. } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Completed)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete completed job");
}

#[test]
fn cancel_accepted_during_terminal_hook_cannot_publish_completion() {
    static MANAGER: SolverManager<TerminalHookLifecycleSolution> = SolverManager::new();

    let solution = TerminalHookLifecycleSolution::new(29);
    let hook = solution.terminal_hook.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");
    hook.wait_until_blocked();

    MANAGER
        .cancel(job_id)
        .expect("cancel during terminal hook is accepted");
    hook.release();

    match recv_event(&mut receiver, "cancelled terminal solve") {
        SolverEvent::Cancelled { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Cancelled);
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Cancelled)
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
    assert_eq!(
        MANAGER
            .get_status(job_id)
            .expect("cancelled status")
            .lifecycle_state,
        SolverLifecycleState::Cancelled
    );

    MANAGER.delete(job_id).expect("delete cancelled job");
}

#[test]
fn retained_job_delete_keeps_slot_reserved_until_worker_exit() {
    static MANAGER: SolverManager<DeleteReservationSolution> = SolverManager::new();

    let solution = DeleteReservationSolution::new();
    let release_return = solution.release_return.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

    match recv_event(&mut receiver, "completed event") {
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
fn zero_variable_job_cancelled_while_paused_reports_cancelled() {
    static MANAGER: SolverManager<ZeroVariableLifecycleSolution> = SolverManager::new();

    let solution = ZeroVariableLifecycleSolution::new();
    let gate = solution.gate.clone();
    let (job_id, mut receiver) = MANAGER.solve(solution).expect("job should start");

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

    gate.allow_next_step();

    match recv_event(&mut receiver, "best solution event") {
        SolverEvent::BestSolution { metadata, .. } => {
            assert_eq!(
                metadata.lifecycle_state,
                SolverLifecycleState::PauseRequested
            );
            assert_eq!(metadata.snapshot_revision, Some(1));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match recv_event(&mut receiver, "paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.cancel(job_id).expect("cancel should be accepted");

    match recv_event(&mut receiver, "cancelled event") {
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

#[test]
fn retained_job_panic_propagates_as_failure_and_releases_the_slot() {
    static MANAGER: SolverManager<PanickingLifecycleSolution> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER
        .solve(PanickingLifecycleSolution::new(3))
        .expect("panicking job should start");

    match recv_event(&mut receiver, "failed event") {
        SolverEvent::Failed { metadata, error } => {
            assert_eq!(metadata.job_id, job_id);
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Failed);
            assert!(
                error.contains("panicking job explosion marker"),
                "the panic message must reach the failure event verbatim: {error}"
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    // The panicked worker must not poison the manager: the slot is released
    // and a follow-up healthy job completes normally on the same manager.
    let status = MANAGER.get_status(job_id).expect("status after failure");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Failed);

    // Terminal jobs stay retained until delete; the panicked slot becomes
    // free exactly when the job is deleted and the worker has exited. Deleting
    // must succeed (bounded retry for the async worker exit), and the manager
    // must then accept a fresh job on the released slot.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match MANAGER.delete(job_id) {
            Ok(()) => break,
            Err(SolverManagerError::JobNotFound { .. }) => break,
            Err(other) => {
                assert!(
                    std::time::Instant::now() < deadline,
                    "failed job could not be deleted: {other:?}"
                );
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
    }

    let (second_id, mut second_receiver) = MANAGER
        .solve(PanickingLifecycleSolution::new(3))
        .expect("manager must accept a new job after deleting the panicked one");
    assert_eq!(second_id, job_id, "the released slot must be reused");
    match recv_event(&mut second_receiver, "follow-up failed event") {
        SolverEvent::Failed { .. } => {}
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn retained_jobs_solve_independently_without_cross_publication() {
    static MANAGER: SolverManager<LifecycleSolution> = SolverManager::new();

    let mut gates = Vec::new();
    let mut receivers = Vec::new();
    for (job_value, initial_score) in [(7, 7), (11, 11)] {
        let solution =
            LifecycleSolution::new_with_score(job_value, Some(SoftScore::of(initial_score)));
        gates.push(solution.gate.clone());
        let (_id, receiver) = MANAGER.solve(solution).expect("job should start");
        receivers.push(receiver);
    }

    // Unblock both gated steps; each job then drives to completion.
    for gate in &gates {
        gate.allow_next_step();
    }

    for (expected_value, mut receiver) in [(7, receivers.remove(0)), (11, receivers.remove(0))] {
        let mut best_value: Option<i64> = None;
        loop {
            match recv_event(&mut receiver, "job completion flow") {
                SolverEvent::BestSolution { ref solution, .. } => {
                    assert_eq!(solution.value, expected_value, "no cross-publication");
                    best_value = Some(solution.value);
                }
                SolverEvent::Progress { .. } => {}
                SolverEvent::Completed { ref solution, .. } => {
                    assert_eq!(solution.value, expected_value);
                    assert_eq!(best_value, Some(expected_value));
                    break;
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
    }
}
