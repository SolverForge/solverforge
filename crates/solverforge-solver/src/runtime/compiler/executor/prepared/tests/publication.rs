use super::*;

#[test]
fn explicit_list_construction_stops_at_a_limit_reached_inside_the_kernel() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for heuristic in [
        ConstructionHeuristicType::ListRoundRobin,
        ConstructionHeuristicType::ListCheapestInsertion,
        ConstructionHeuristicType::ListRegretInsertion,
    ] {
        let mut phase = construction(heuristic);
        let PhaseConfig::ConstructionHeuristic(config) = &mut phase else {
            unreachable!("construction helper must return a construction phase");
        };
        config.termination = Some(TerminationConfig {
            step_count_limit: Some(1),
            ..TerminationConfig::default()
        });
        let config = SolverConfig {
            phases: vec![phase],
            ..SolverConfig::default()
        };
        let mut prepared = executor(&config)
            .instantiate()
            .expect("explicit list construction prepares");
        let construction = match prepared.phases.remove(0) {
            PreparedRuntimePhase::Construction(construction) => construction,
            _ => panic!("configured phase should remain construction"),
        };
        let input = plan((1..=16).collect(), vec![Vec::new(), Vec::new()]);
        let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
        let mut scope = SolverScope::new(director);

        assert!(
            execute_prepared_construction(&mut prepared, &construction, false, &mut scope)
                .expect("explicit list construction executes until its phase limit")
                .ran()
        );
        assert_eq!(
            scope.working_solution().routes.iter().flatten().count(),
            1,
            "{heuristic:?} must stop after the first committed step"
        );
    }
}

#[test]
fn required_construction_stops_at_an_ordinary_phase_limit() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut phase = construction(ConstructionHeuristicType::ListRoundRobin);
    let PhaseConfig::ConstructionHeuristic(config) = &mut phase else {
        unreachable!("construction helper must return a construction phase");
    };
    config.termination = Some(TerminationConfig {
        step_count_limit: Some(0),
        ..TerminationConfig::default()
    });
    let config = SolverConfig {
        phases: vec![phase],
        ..SolverConfig::default()
    };
    let mut prepared = executor(&config)
        .instantiate()
        .expect("required construction prepares");
    let construction = match prepared.phases.remove(0) {
        PreparedRuntimePhase::Construction(construction) => construction,
        _ => panic!("configured phase should remain construction"),
    };
    let input = plan((1..=8).collect(), vec![Vec::new(), Vec::new()]);
    let director = ScoreDirector::simple(input, descriptor(), |plan, _| entity_count(plan));
    let mut scope = SolverScope::new(director);

    let execution = execute_prepared_construction(&mut prepared, &construction, true, &mut scope)
        .expect("required construction observes ordinary phase limits");
    assert!(!execution.ran());
    assert_eq!(scope.working_solution().routes.iter().flatten().count(), 0);
}

#[test]
fn compiled_runtime_fails_incomplete_list_work_without_publishing_a_best_solution() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut phase = construction(ConstructionHeuristicType::ListRoundRobin);
    let PhaseConfig::ConstructionHeuristic(config) = &mut phase else {
        unreachable!("construction helper must return a construction phase");
    };
    config.termination = Some(TerminationConfig {
        step_count_limit: Some(0),
        ..TerminationConfig::default()
    });
    let config = SolverConfig {
        phases: vec![phase],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let runner =
        CompiledRuntimePhaseRunner::try_new(executor).expect("list runtime runner must prepare");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let director = ScoreDirector::simple(
        plan((1..=8).collect(), vec![Vec::new(), Vec::new()]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let solver = Solver::new((runner,))
        .with_config(config)
        .with_progress_callback(move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        });

    let payload = std::panic::catch_unwind(AssertUnwindSafe(|| solver.solve(director)))
        .expect_err("incomplete mandatory list work must fail");
    let error = take_runtime_execution_failure(payload)
        .expect("compiled-runtime incompleteness must use the typed failure channel");
    assert!(matches!(
        error,
        RuntimeBuildError::Execution { phase_index: 0, .. }
    ));
    assert!(error.to_string().contains("8 unassigned element(s)"));
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 0);
}

#[test]
fn compiled_runtime_publishes_only_after_list_work_is_complete() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListRoundRobin)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let runner =
        CompiledRuntimePhaseRunner::try_new(executor).expect("list runtime runner must prepare");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let director = ScoreDirector::simple(
        plan(vec![1, 2, 3], vec![Vec::new()]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let solver = Solver::new((runner,))
        .with_config(config)
        .with_progress_callback(move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                let solution = progress
                    .solution
                    .expect("published best solution must carry a solution");
                assert_eq!(solution.routes.iter().flatten().count(), 3);
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        });

    let result = solver.solve(director);

    assert_eq!(result.solution.routes.iter().flatten().count(), 3);
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 1);
}

#[test]
fn mandatory_completion_gate_recloses_if_work_becomes_incomplete() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListRoundRobin)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let bindings = executor.graph().default_bindings().clone();
    let mut execution = executor
        .instantiate()
        .expect("list runtime execution must prepare");
    let published_sizes = Arc::new(Mutex::new(Vec::new()));
    let observed_sizes = Arc::clone(&published_sizes);
    let director = ScoreDirector::simple(
        plan(vec![1, 2, 3], vec![vec![1, 2, 3]]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let mut scope = SolverScope::new_with_callback(
        director,
        move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_sizes.lock().unwrap().push(
                    progress
                        .solution
                        .expect("best event carries a solution")
                        .routes
                        .iter()
                        .flatten()
                        .count(),
                );
            }
        },
        None,
        None,
    );
    scope.defer_best_solution_publication();
    scope.initialize_working_solution_as_best();
    let mut completion_published = false;

    assert!(publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("complete state publishes"));

    scope.replace_working_solution_and_reinitialize(plan(vec![1, 2, 3], vec![vec![1, 2]]));
    assert!(!publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("incomplete state recloses the gate"));
    scope.report_best_solution();

    scope.replace_working_solution_and_reinitialize(plan(vec![1, 2, 3], vec![vec![1, 2, 3]]));
    assert!(publish_if_mandatory_complete(
        &mut execution,
        &bindings,
        &mut completion_published,
        0,
        &mut scope,
    )
    .expect("restored complete state republishes"));

    assert_eq!(*published_sizes.lock().unwrap(), vec![3, 3]);
}

#[test]
fn already_complete_list_work_can_terminate_by_config_and_publish() {
    let _guard = TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let config = SolverConfig {
        phases: vec![construction(ConstructionHeuristicType::ListRoundRobin)],
        ..SolverConfig::default()
    };
    let executor = executor(&config);
    let runner =
        CompiledRuntimePhaseRunner::try_new(executor).expect("list runtime runner must prepare");
    let best_solution_events = Arc::new(AtomicUsize::new(0));
    let observed_events = Arc::clone(&best_solution_events);
    let director = ScoreDirector::simple(
        plan(vec![1, 2, 3], vec![vec![1, 2, 3]]),
        descriptor(),
        |plan, _| entity_count(plan),
    );
    let solver = Solver::new((runner,))
        .with_config(config)
        .with_time_limit(Duration::ZERO)
        .with_progress_callback(move |progress: SolverProgressRef<'_, Plan>| {
            if progress.kind == SolverProgressKind::BestSolution {
                observed_events.fetch_add(1, Ordering::SeqCst);
            }
        });

    let result = solver.solve(director);

    assert_eq!(
        result.terminal_reason,
        crate::manager::SolverTerminalReason::TerminatedByConfig
    );
    assert_eq!(result.solution.routes, vec![vec![1, 2, 3]]);
    assert_eq!(best_solution_events.load(Ordering::SeqCst), 1);
}
