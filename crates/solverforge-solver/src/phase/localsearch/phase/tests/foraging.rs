#[test]
fn accepted_count_one_evaluates_one_accepted_move_per_step() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreFieldSelector::new([1, 2, 3]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1, false);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(1))
    );
}

#[test]
fn cancellation_before_next_candidate_does_not_commit_selected_move() {
    let terminate = Box::leak(Box::new(AtomicBool::new(false)));
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director).with_terminate(Some(terminate));
    solver_scope.start_solving();

    let move_selector = CancelOnDoableSelector { terminate };
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2, false);
    let mut phase: LocalSearchPhase<_, CancelOnDoableMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_applied, 0);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(0))
    );
}

#[test]
fn accepted_count_limit_picks_best_of_accepted_horizon() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreFieldSelector::new([1, 3, 2]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2, false);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 2);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(3))
    );
}

#[test]
fn config_move_count_interrupt_commits_best_already_accepted_move() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();
    solver_scope.inphase_move_count_limit = Some(1);

    let move_selector = ScoreFieldSelector::new([1, 3]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2, false);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 1);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(1))
    );
}

#[test]
fn best_score_forager_still_scans_full_neighborhood() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreFieldSelector::new([1, 3, 2]);
    let acceptor = HillClimbingAcceptor::new();
    let forager: BestScoreForager<_> = BestScoreForager::new(false);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 3);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(3))
    );
}

#[test]
fn score_improvement_required_move_rejects_worse_before_acceptor() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = ScoreImprovementRequiredSelector::new([-5, 3]);
    let acceptor = AlwaysAcceptAcceptor;
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1, false);
    let mut phase: LocalSearchPhase<_, ScoreImprovementRequiredMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 2);
    assert_eq!(solver_scope.stats().moves_applied, 1);
    assert_eq!(
        solver_scope.working_solution().score,
        Some(SoftScore::of(3))
    );
}
