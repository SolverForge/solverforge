
#[test]
fn test_construction_phase_reports_one_best_solution_on_improvement() {
    let director = create_simple_nqueens_director(4);
    let best_events = Arc::new(AtomicUsize::new(0));
    let best_events_for_callback = Arc::clone(&best_events);
    let mut solver_scope = SolverScope::new_with_callback(
        director,
        move |progress: crate::scope::SolverProgressRef<'_, NQueensSolution>| {
            if progress.kind == crate::scope::SolverProgressKind::BestSolution {
                best_events_for_callback.fetch_add(1, Ordering::SeqCst);
            }
        },
        None,
        None,
    );
    solver_scope.start_solving();

    let values: Vec<i64> = (0..4).collect();
    let placer = create_placer(values);
    let forager = FirstFitForager::new();
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    assert_eq!(best_events.load(Ordering::SeqCst), 1);
}

#[test]
fn construction_reports_progress_before_a_long_phase_ends() {
    let gate = BlockingEvaluationGate::delaying(Duration::from_millis(400));
    let progress_events = Arc::new(AtomicUsize::new(0));
    let progress_events_for_callback = Arc::clone(&progress_events);
    let solution = ConstructionPauseSolution::with_entity_count(3, Some(gate.clone()));
    let mut solver_scope = SolverScope::new_with_callback(
        ConstructionPauseDirector::new(solution),
        move |progress: crate::scope::SolverProgressRef<'_, ConstructionPauseSolution>| {
            if progress.kind == crate::scope::SolverProgressKind::Progress {
                assert!(progress.telemetry.step_count > 0);
                assert!(progress.telemetry.moves_evaluated > 0);
                progress_events_for_callback.fetch_add(1, Ordering::SeqCst);
            }
        },
        None,
        None,
    );
    solver_scope.start_solving();

    let mut phase = ConstructionHeuristicPhase::new(
        ConstructionPausePlacer::new(Some(gate)),
        FirstFitForager::new(),
    );
    phase.solve(&mut solver_scope);

    assert!(
        progress_events.load(Ordering::SeqCst) > 0,
        "construction ended without a progress event"
    );
}

#[test]
fn test_construction_empty_solution() {
    let director = create_simple_nqueens_director(0);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let values: Vec<i64> = vec![];
    let placer = create_placer(values);
    let forager = FirstFitForager::new();
    let mut phase = ConstructionHeuristicPhase::new(placer, forager);

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 0);
}

#[test]
fn keep_current_pause_snapshot_has_committed_score() {
    static MANAGER: SolverManager<ConstructionPauseSolution> = SolverManager::new();

    let gate = BlockingEvaluationGate::new(1);
    let (job_id, mut receiver) = MANAGER
        .solve(ConstructionPauseSolution::keep_current_pause(Some(
            gate.clone(),
        )))
        .expect("paused keep-current job should start");

    gate.wait_until_blocked();
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

    gate.release();

    let paused_snapshot_revision = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            metadata
                .snapshot_revision
                .expect("paused snapshot revision")
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let paused_snapshot = MANAGER
        .get_snapshot(job_id, Some(paused_snapshot_revision))
        .expect("paused snapshot");
    assert_eq!(paused_snapshot.current_score, Some(SoftScore::of(0)));
    assert_eq!(paused_snapshot.best_score, None);
    assert_eq!(paused_snapshot.solution.entities[0].value, None);
    assert_eq!(paused_snapshot.solution.score(), Some(SoftScore::of(0)));

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(solution.entities[0].value, None);
            assert_eq!(solution.score(), Some(SoftScore::of(0)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete paused job");
}

#[test]
fn test_construction_resume_retries_interrupted_placement() {
    static MANAGER: SolverManager<ConstructionPauseSolution> = SolverManager::new();

    let (uninterrupted_job_id, mut uninterrupted_receiver) = MANAGER
        .solve(ConstructionPauseSolution::new(None))
        .expect("uninterrupted job should start");

    let uninterrupted_value = match uninterrupted_receiver
        .blocking_recv()
        .expect("uninterrupted completed event")
    {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(solution.entities[0].value, Some(64));
            assert_eq!(solution.score(), Some(SoftScore::of(64)));
            solution.entities[0].value
        }
        other => panic!("unexpected event: {other:?}"),
    };

    MANAGER
        .delete(uninterrupted_job_id)
        .expect("delete uninterrupted job");

    let gate = BlockingEvaluationGate::new(1);
    let (job_id, mut receiver) = MANAGER
        .solve(ConstructionPauseSolution::new(Some(gate.clone())))
        .expect("paused job should start");

    gate.wait_until_blocked();
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

    gate.release();

    let paused_snapshot_revision = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Paused);
            metadata
                .snapshot_revision
                .expect("paused snapshot revision")
        }
        other => panic!("unexpected event: {other:?}"),
    };

    let paused_snapshot = MANAGER
        .get_snapshot(job_id, Some(paused_snapshot_revision))
        .expect("paused snapshot");
    assert_eq!(paused_snapshot.solution.entities[0].value, None);

    MANAGER.resume(job_id).expect("resume should be accepted");

    match receiver.blocking_recv().expect("resumed event") {
        SolverEvent::Resumed { metadata } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Solving);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { metadata, solution } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
            assert_eq!(solution.entities[0].value, uninterrupted_value);
            assert_eq!(solution.score(), Some(SoftScore::of(64)));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    MANAGER.delete(job_id).expect("delete resumed job");
}
