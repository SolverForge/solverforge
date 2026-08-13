use super::*;

#[test]
fn cancellation_retains_complete_working_solution_over_better_incomplete_best() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListClarkeWright)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let bindings = executor.graph().default_bindings().clone();
    let mut execution = executor
        .instantiate()
        .expect("Clarke-Wright runtime execution prepares");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let incomplete = plan(vec![1, 2, 3], vec![Vec::new()]);
    let director = ScoreDirector::simple(incomplete.clone(), descriptor(), |plan, _| {
        entity_count(plan)
    });
    let mut scope = SolverScope::new_with_callback(
        director,
        move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        },
        None,
        None,
    );
    scope.defer_best_solution_publication();
    scope.set_best_solution(incomplete, SoftScore::of(1));
    scope.mutate(|director| director.working_solution_mut().routes = vec![vec![1, 2, 3]]);
    scope.mark_cancelled();
    let mut completion_published = false;

    assert!(!publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("cancellation completion check succeeds"));

    assert_eq!(
        scope.terminal_reason(),
        crate::manager::SolverTerminalReason::Cancelled
    );
    assert_eq!(
        scope
            .best_solution()
            .expect("complete working state replaces incomplete prior best")
            .routes,
        vec![vec![1, 2, 3]]
    );
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 0);
}
