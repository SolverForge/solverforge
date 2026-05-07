use super::*;

#[test]
fn descriptor_first_fit_ignores_value_order_hook() {
    fn prefer_worker_one(_solution: &dyn std::any::Any, _entity_index: usize, value: usize) -> i64 {
        if value == 1 {
            0
        } else {
            10
        }
    }

    let descriptor = queue_descriptor(None, Some(prefer_worker_one));
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![QueueTask {
            worker_idx: None,
            preferred_worker: 0,
            allowed_workers: vec![0, 1],
        }],
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
}

#[test]
fn descriptor_allocate_to_value_from_queue_consumes_value_order_hook() {
    fn prefer_worker_one(_solution: &dyn std::any::Any, _entity_index: usize, value: usize) -> i64 {
        if value == 1 {
            0
        } else {
            10
        }
    }

    let descriptor = queue_descriptor(None, Some(prefer_worker_one));
    let plan = QueuePlan {
        workers: vec![Worker, Worker],
        tasks: vec![QueueTask {
            worker_idx: None,
            preferred_worker: 0,
            allowed_workers: vec![0, 1],
        }],
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

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(1));
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

#[test]
fn descriptor_live_refresh_generates_candidates_for_selected_slot_only() {
    let descriptor = queue_descriptor(None, Some(queue_value_load_key));
    let worker_count = 5;
    let task_count = 4;
    let plan = QueuePlan {
        workers: (0..worker_count).map(|_| Worker).collect(),
        tasks: (0..task_count)
            .map(|_| QueueTask {
                worker_idx: None,
                preferred_worker: 0,
                allowed_workers: (0..worker_count).collect(),
            })
            .collect(),
        assignment_log: Vec::new(),
        score: None,
    };
    let director = QueueScoreDirector::new(plan, descriptor.clone());
    let mut solver_scope = SolverScope::new(director);
    solver_scope.start_solving();

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: None,
        construction_heuristic_type: ConstructionHeuristicType::WeakestFit,
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<QueuePlan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.stats().step_count, task_count as u64);
    assert_eq!(
        solver_scope.stats().moves_generated,
        (worker_count * task_count) as u64
    );
    assert_eq!(
        solver_scope
            .working_solution()
            .tasks
            .iter()
            .filter(|task| task.worker_idx.is_some())
            .count(),
        task_count
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
