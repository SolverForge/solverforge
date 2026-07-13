#[test]
fn test_local_search_records_selector_open_time_as_generation_time() {
    let director = create_minimal_director();
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let move_selector = SlowOpenSelector(Duration::from_millis(20));
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1, false);
    let mut phase: LocalSearchPhase<_, NoopMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert!(solver_scope.stats().generation_time() >= Duration::from_millis(20));
    assert_eq!(solver_scope.stats().moves_generated, 1);
}

#[test]
fn local_search_reports_progress_without_an_8192_move_batch() {
    let progress_events = Arc::new(AtomicUsize::new(0));
    let progress_events_for_callback = Arc::clone(&progress_events);
    let director = create_minimal_director();
    let mut solver_scope = SolverScope::new_with_callback(
        director,
        move |progress: crate::scope::SolverProgressRef<'_, TestSolution>| {
            if progress.kind == crate::scope::SolverProgressKind::Progress {
                progress_events_for_callback.fetch_add(1, Ordering::SeqCst);
            }
        },
        None,
        None,
    );
    solver_scope.start_solving();

    let move_selector = SlowOpenSelector(Duration::from_millis(400));
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(1, false);
    let mut phase: LocalSearchPhase<_, NoopMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(3));

    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().moves_evaluated, 3);
    assert!(
        progress_events.load(Ordering::SeqCst) > 0,
        "local search ended without reporting its completed steps"
    );
}

#[test]
fn local_search_opens_selector_with_stream_context() {
    let director = ScoreFieldDirector::new();
    let mut solver_scope = SolverScope::new(director).with_seed(7);
    solver_scope.start_solving();

    let saw_context = Box::leak(Box::new(AtomicBool::new(false)));
    let step_index = Box::leak(Box::new(AtomicU64::new(u64::MAX)));
    let accepted_limit = Box::leak(Box::new(AtomicUsize::new(usize::MAX)));
    let move_selector = ContextSpySelector {
        saw_context,
        step_index,
        accepted_limit,
    };
    let acceptor = HillClimbingAcceptor::new();
    let forager: AcceptedCountForager<_> = AcceptedCountForager::new(2, false);
    let mut phase: LocalSearchPhase<_, ScoreFieldMove, _, _, _> =
        LocalSearchPhase::new(move_selector, acceptor, forager, Some(1));

    phase.solve(&mut solver_scope);

    assert!(saw_context.load(Ordering::SeqCst));
    assert_eq!(step_index.load(Ordering::SeqCst), 0);
    assert_eq!(accepted_limit.load(Ordering::SeqCst), 2);
}
