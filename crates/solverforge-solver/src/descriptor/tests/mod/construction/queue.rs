use super::*;

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
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
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
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
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
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
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
