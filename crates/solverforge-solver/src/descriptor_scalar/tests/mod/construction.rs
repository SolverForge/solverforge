
#[test]
fn solution_level_value_range_generates_scalar_work() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };

    assert!(scalar_work_remaining(
        &descriptor,
        Some("Task"),
        Some("worker_idx"),
        &plan
    ));
}

#[test]
fn solution_level_value_range_builds_change_moves() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let selector = build_descriptor_move_selector::<Plan>(None, &descriptor, None);

    assert_eq!(selector.size(&director), 3);
}

#[test]
fn descriptor_change_selector_adds_to_none_for_assigned_optional_variables() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task {
            worker_idx: Some(1),
        }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let selector = build_descriptor_move_selector::<Plan>(None, &descriptor, None);
    let moves: Vec<_> = selector.iter_moves(&director).collect();

    assert_eq!(selector.size(&director), 4);
    assert_eq!(moves.len(), 4);
}

#[test]
fn descriptor_first_fit_required_slot_still_assigns_first_doable_value() {
    let descriptor = descriptor_with_allows_unassigned(false);
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }, Task { worker_idx: None }],
        score: None,
    };
    let director = ScoreDirector::simple(plan, descriptor.clone(), |s, _| s.tasks.len());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert!(solver_scope
        .working_solution()
        .tasks
        .iter()
        .all(|task| task.worker_idx == Some(0)));
    assert_eq!(solver_scope.stats().moves_accepted, 2);
}

#[test]
fn descriptor_first_fit_optional_slot_keeps_none_when_baseline_is_not_beaten() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, -1, -2],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(0))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 0);
    assert_eq!(solver_scope.stats().step_count, 1);
}

#[test]
fn descriptor_first_fit_optional_slot_skips_worse_candidate_for_later_improvement() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [-5, 7, -1],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn descriptor_first_fit_optional_slot_takes_first_improving_candidate() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::with_mode(
        plan,
        descriptor.clone(),
        PlanScoreMode::ByWorker {
            unassigned_score: 0,
            assigned_scores: [7, -5, 3],
        },
    );
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut phase = build_descriptor_construction::<Plan>(None, &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(7))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn descriptor_best_fit_assigns_optional_variable_when_candidate_improves_score() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: Some(3),
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<Plan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn descriptor_completed_optional_none_is_skipped_by_later_construction_passes() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director =
        PlanScoreDirector::with_mode(plan, descriptor.clone(), PlanScoreMode::PreferUnassigned);
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let best_fit_config = ConstructionHeuristicConfig {
        value_candidate_limit: Some(3),
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut best_fit_phase =
        build_descriptor_construction::<Plan>(Some(&best_fit_config), &descriptor);
    best_fit_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);

    let mut first_fit_phase = build_descriptor_construction::<Plan>(None, &descriptor);
    first_fit_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);
    assert_eq!(solver_scope.stats().step_count, 1);
    assert_eq!(solver_scope.stats().moves_accepted, 0);
}

#[test]
fn descriptor_reopened_optional_slot_is_revisited_by_later_construction() {
    let descriptor = descriptor();
    let plan = Plan {
        workers: vec![Worker, Worker, Worker],
        tasks: vec![Task { worker_idx: None }],
        score: None,
    };
    let director = PlanScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let mut first_fit_phase = build_descriptor_construction::<Plan>(None, &descriptor);
    first_fit_phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));

    solver_scope
        .score_director_mut()
        .set_score_mode(PlanScoreMode::PreferUnassigned);

    let move_selector = build_descriptor_move_selector::<Plan>(None, &descriptor, None);
    let mut local_search = LocalSearchPhase::new(
        move_selector,
        HillClimbingAcceptor::new(),
        FirstAcceptedForager::new(),
        Some(1),
    );
    local_search.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, None);

    solver_scope
        .score_director_mut()
        .set_score_mode(PlanScoreMode::AllAssignedBonus);

    let mut reconstruct = build_descriptor_construction::<Plan>(None, &descriptor);
    reconstruct.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
}

#[test]
fn descriptor_first_fit_decreasing_reevaluates_entity_order_each_step() {
    let descriptor = queue_descriptor(Some(queue_entity_order_key), None);
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 1,
                allowed_workers: vec![1],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0],
            },
        ],
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::FirstFitDecreasing,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.working_solution().assignment_log,
        vec![0, 2, 1]
    );
}

#[test]
fn descriptor_allocate_entity_from_queue_reevaluates_entity_order_each_step() {
    let descriptor = queue_descriptor(Some(queue_entity_order_key), None);
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 1,
                allowed_workers: vec![1],
            },
        ],
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::AllocateEntityFromQueue,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.working_solution().assignment_log,
        vec![0, 2, 1]
    );
}

#[test]
fn descriptor_allocate_to_value_from_queue_uses_live_value_order() {
    let descriptor = queue_descriptor(None, Some(queue_value_load_key));
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
        ],
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::AllocateToValueFromQueue,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope
            .working_solution()
            .tasks
            .iter()
            .map(|task| task.worker_idx)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)]
    );
}

#[test]
fn descriptor_weakest_fit_uses_live_value_order_key() {
    let descriptor = queue_descriptor(None, Some(queue_value_load_key));
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
        ],
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::WeakestFit,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope
            .working_solution()
            .tasks
            .iter()
            .map(|task| task.worker_idx)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)]
    );
}

#[test]
fn descriptor_strongest_fit_uses_live_value_order_key() {
    let descriptor = queue_descriptor(None, Some(queue_value_balance_key));
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
            QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: vec![0, 1],
            },
        ],
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::StrongestFit,
        target: VariableTargetConfig::default(),
        k: 1,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope
            .working_solution()
            .tasks
            .iter()
            .map(|task| task.worker_idx)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1)]
    );
}
