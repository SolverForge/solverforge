
use super::*;

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
fn descriptor_first_fit_forced_optional_slot_assigns_first_candidate_even_when_worse() {
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

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: Some(3),
        construction_heuristic_type: ConstructionHeuristicType::FirstFit,
        construction_obligation: solverforge_config::ConstructionObligation::AssignWhenCandidateExists,
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<Plan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(0));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-5))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
}

#[test]
fn descriptor_best_fit_forced_optional_slot_assigns_best_candidate_even_when_worse() {
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

    let config = ConstructionHeuristicConfig {
        value_candidate_limit: Some(3),
        construction_heuristic_type: ConstructionHeuristicType::CheapestInsertion,
        construction_obligation: solverforge_config::ConstructionObligation::AssignWhenCandidateExists,
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
        termination: None,
    };
    let mut phase = build_descriptor_construction::<Plan>(Some(&config), &descriptor);
    phase.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().tasks[0].worker_idx, Some(1));
    assert_eq!(
        solver_scope.current_score().copied(),
        Some(SoftScore::of(-1))
    );
    assert_eq!(solver_scope.stats().moves_accepted, 1);
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
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
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
        construction_obligation: Default::default(),
        target: VariableTargetConfig::default(),
        k: 1,
        group_name: None,
        group_candidate_limit: None,
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
