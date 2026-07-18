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

impl ConstructionPauseSolution {
    fn best_fit_interrupted_pull(gate: BlockingEvaluationGate) -> Self {
        Self {
            eval_gate: Some(gate),
            solvable_mode: ConstructionPauseSolvableMode::BestFitInterruptedPull,
            ..Self::new(None)
        }
    }
}

fn solve_best_fit_interrupted_pull(
    solver_scope: &mut SolverScope<'_, ConstructionPauseSolution, ConstructionPauseDirector>,
    gate: Option<BlockingEvaluationGate>,
) {
    let placer = InterruptAfterRetainedCandidatePlacer {
        gate: gate.expect("interrupted-pull solve requires a generation gate"),
    };
    let mut phase = ConstructionHeuristicPhase::new(placer, BestFitForager::new());
    phase.solve(solver_scope);
}

struct InterruptAfterRetainedCandidateCursor {
    candidate: Option<ConstructionPauseMove>,
    pulled: bool,
    gate: BlockingEvaluationGate,
}

impl InterruptAfterRetainedCandidateCursor {
    fn new(gate: BlockingEvaluationGate) -> Self {
        Self {
            candidate: Some(ConstructionPauseMove::new(0, 7, true, None)),
            pulled: false,
            gate,
        }
    }
}

impl MoveCursor<ConstructionPauseSolution, ConstructionPauseMove>
    for InterruptAfterRetainedCandidateCursor
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        if self.pulled {
            None
        } else {
            self.pulled = true;
            Some(CandidateId::new(0))
        }
    }

    fn next_candidate_with_control<ShouldStop>(
        &mut self,
        should_stop: &mut ShouldStop,
    ) -> Option<CandidateId>
    where
        ShouldStop: FnMut() -> bool,
    {
        if !self.pulled {
            return self.next_candidate();
        }
        self.gate.on_evaluation();
        let _ = should_stop();
        None
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, ConstructionPauseSolution, ConstructionPauseMove>> {
        if id.index() != 0 {
            return None;
        }
        self.candidate.as_ref().map(MoveCandidateRef::Borrowed)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ConstructionPauseMove {
        assert_eq!(id.index(), 0);
        self.candidate
            .take()
            .expect("retained construction candidate must remain live")
    }
}

#[derive(Clone, Debug)]
struct InterruptAfterRetainedCandidatePlacer {
    gate: BlockingEvaluationGate,
}

impl EntityPlacerCursor<ConstructionPauseSolution, ConstructionPauseMove>
    for InterruptAfterRetainedCandidatePlacer
{
    type CandidateCursor = InterruptAfterRetainedCandidateCursor;

    fn next_placement<D, IsCompleted, ShouldStop>(
        &mut self,
        _score_director: &D,
        mut is_completed: IsCompleted,
        mut should_stop: ShouldStop,
    ) -> Option<Placement<ConstructionPauseSolution, ConstructionPauseMove, Self::CandidateCursor>>
    where
        D: Director<ConstructionPauseSolution>,
        IsCompleted: FnMut(
            &Placement<ConstructionPauseSolution, ConstructionPauseMove, Self::CandidateCursor>,
        ) -> bool,
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            return None;
        }
        let placement = Placement::new(
            EntityReference::new(0, 0),
            InterruptAfterRetainedCandidateCursor::new(self.gate.clone()),
        )
        .with_slot_id(crate::phase::construction::ConstructionSlotId::new(0, 0));
        (!is_completed(&placement)).then_some(placement)
    }
}

impl EntityPlacer<ConstructionPauseSolution, ConstructionPauseMove>
    for InterruptAfterRetainedCandidatePlacer
{
    type Cursor<'a>
        = Self
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<ConstructionPauseSolution>>(
        &'a self,
        _score_director: &D,
    ) -> Self::Cursor<'a> {
        Self {
            gate: self.gate.clone(),
        }
    }
}

#[test]
fn paused_candidate_pull_restarts_without_committing_or_completing_retained_selection() {
    static MANAGER: SolverManager<ConstructionPauseSolution> = SolverManager::new();

    let gate = BlockingEvaluationGate::new(1);
    let (job_id, mut receiver) = MANAGER
        .solve(ConstructionPauseSolution::best_fit_interrupted_pull(
            gate.clone(),
        ))
        .expect("interrupted-pull construction job should start");
    gate.wait_until_blocked();
    MANAGER.pause(job_id).expect("pause should be accepted");
    assert!(matches!(
        receiver.blocking_recv().expect("pause requested event"),
        SolverEvent::PauseRequested { .. }
    ));
    gate.release();

    let paused_revision = match receiver.blocking_recv().expect("paused event") {
        SolverEvent::Paused { metadata } => metadata
            .snapshot_revision
            .expect("paused snapshot revision"),
        other => panic!("unexpected event: {other:?}"),
    };
    let paused_snapshot = MANAGER
        .get_snapshot(job_id, Some(paused_revision))
        .expect("paused snapshot");

    MANAGER.resume(job_id).expect("resume should be accepted");
    assert!(matches!(
        receiver.blocking_recv().expect("resumed event"),
        SolverEvent::Resumed { .. }
    ));
    let completed_solution = match receiver.blocking_recv().expect("completed event") {
        SolverEvent::Completed { solution, .. } => solution,
        other => panic!("unexpected event: {other:?}"),
    };
    MANAGER.delete(job_id).expect("delete resumed job");

    assert_eq!(paused_snapshot.solution.entities[0].value, None);
    assert_eq!(paused_snapshot.telemetry.moves_generated, 1);
    assert_eq!(paused_snapshot.telemetry.moves_evaluated, 1);
    assert_eq!(paused_snapshot.telemetry.moves_accepted, 0);
    assert_eq!(paused_snapshot.telemetry.moves_applied, 0);
    assert_eq!(completed_solution.entities[0].value, Some(7));
}
