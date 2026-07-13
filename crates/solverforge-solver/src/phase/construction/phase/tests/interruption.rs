#[test]
fn time_limit_after_first_fit_selection_commits_before_termination() {
    let gate = BlockingEvaluationGate::delaying(Duration::from_millis(100));
    let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    solver_scope.set_time_limit(Duration::from_millis(20));

    let placer =
        ScoredConstructionPlacer::new((0..66).collect(), false).with_eval_gate(Some(gate));
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn time_limit_after_first_feasible_selection_commits_before_termination() {
    let gate = BlockingEvaluationGate::delaying(Duration::from_millis(100));
    let director = ConstructionPauseDirector::with_score_mode(
        ConstructionPauseSolution::new(None),
        ConstructionPauseScoreMode::AssignedSum {
            unassigned_score: -2,
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    solver_scope.set_time_limit(Duration::from_millis(20));

    let placer =
        ScoredConstructionPlacer::new((0..66).collect(), true).with_eval_gate(Some(gate));
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFeasibleForager::new());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

fn assert_direct_limit_commits_and_terminates(
    configure: impl FnOnce(&mut SolverScope<'static, ConstructionPauseSolution, ConstructionPauseDirector>),
    keep_current_legal: bool,
) {
    let director = ConstructionPauseDirector::with_score_mode(
        ConstructionPauseSolution::new(None),
        ConstructionPauseScoreMode::AssignedSum {
            unassigned_score: -2,
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    configure(&mut solver_scope);

    let placer = ScoredConstructionPlacer::new(vec![0, 1], keep_current_legal);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(solver_scope.stats().moves_generated, 1);
    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn direct_step_limit_terminates_after_committing_selection() {
    assert_direct_limit_commits_and_terminates(
        |scope| scope.inphase_step_count_limit = Some(1),
        false,
    );
}

#[test]
fn direct_move_limit_terminates_after_committing_selection() {
    assert_direct_limit_commits_and_terminates(
        |scope| scope.inphase_move_count_limit = Some(1),
        false,
    );
}

#[test]
fn direct_score_calculation_limit_terminates_after_committing_selection() {
    assert_direct_limit_commits_and_terminates(
        |scope| scope.inphase_score_calc_count_limit = Some(2),
        true,
    );
}

#[test]
fn shared_move_budget_commits_selection_before_child_termination() {
    let parent_director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut parent_scope = SolverScope::new(parent_director);
    parent_scope.inphase_move_count_limit = Some(1);
    let phase_budget = parent_scope.child_phase_budget();
    let child_config = parent_scope.child_config(Some(&phase_budget));

    let child_director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
    let mut child_scope = child_config.build_scope(child_director, 0);
    child_scope.start_solving();
    let placer = ScoredConstructionPlacer::new((0..66).collect(), false);
    let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());
    phase.solve(&mut child_scope);

    assert_eq!(child_scope.working_solution().entities[0].value, Some(0));
    assert_eq!(child_scope.stats().moves_generated, 1);
    assert_eq!(child_scope.stats().moves_evaluated, 1);
    assert_eq!(child_scope.stats().moves_accepted, 1);
    assert_eq!(child_scope.stats().moves_applied, 1);
    assert_eq!(
        child_scope.terminal_reason(),
        SolverTerminalReason::TerminatedByConfig
    );
}

#[test]
fn cancellation_after_decisive_evaluation_keeps_the_committed_selection() {
    let gate = BlockingEvaluationGate::new(1);
    let terminate = AtomicBool::new(false);

    rayon::scope(|scope| {
        scope.spawn(|_| {
            let director = ConstructionPauseDirector::new(ConstructionPauseSolution::new(None));
            let mut solver_scope = SolverScope::new(director).with_terminate(Some(&terminate));
            solver_scope.start_solving();
            let placer = ScoredConstructionPlacer::new((0..66).collect(), false)
                .with_eval_gate(Some(gate.clone()));
            let mut phase = ConstructionHeuristicPhase::new(placer, FirstFitForager::new());
            phase.solve(&mut solver_scope);

            assert_eq!(solver_scope.working_solution().entities[0].value, Some(0));
            assert_eq!(solver_scope.stats().moves_generated, 1);
            assert_eq!(solver_scope.stats().moves_applied, 1);
            assert_eq!(solver_scope.terminal_reason(), SolverTerminalReason::Cancelled);
        });

        gate.wait_until_blocked();
        terminate.store(true, Ordering::SeqCst);
        gate.release();
    });
}

#[test]
fn pause_after_decisive_evaluation_snapshots_the_committed_selection() {
    static MANAGER: SolverManager<ConstructionPauseSolution> = SolverManager::new();

    let gate = BlockingEvaluationGate::new(1);
    let (job_id, mut receiver) = MANAGER
        .solve(ConstructionPauseSolution::decisive(gate.clone()))
        .expect("decisive construction job should start");
    gate.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");

    assert!(matches!(
        receiver.blocking_recv().expect("pause requested event"),
        SolverEvent::PauseRequested { .. }
    ));
    gate.release();

    let revision = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => metadata
            .snapshot_revision
            .expect("paused snapshot revision"),
        other => panic!("unexpected event: {other:?}"),
    };
    let snapshot = MANAGER
        .get_snapshot(job_id, Some(revision))
        .expect("paused snapshot");
    assert_eq!(snapshot.solution.entities[0].value, Some(0));
    assert_eq!(snapshot.telemetry.moves_generated, 1);
    assert_eq!(snapshot.telemetry.moves_applied, 1);

    MANAGER.resume(job_id).expect("resume should be accepted");
    assert!(matches!(
        receiver.blocking_recv().expect("resumed event"),
        SolverEvent::Resumed { .. }
    ));
    match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { solution, .. } => {
            assert_eq!(solution.entities[0].value, Some(0));
        }
        other => panic!("unexpected event: {other:?}"),
    }
    MANAGER.delete(job_id).expect("delete decisive job");
}
