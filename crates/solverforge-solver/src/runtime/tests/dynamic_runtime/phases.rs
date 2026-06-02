#[test]
fn dynamic_scalar_slot_runs_through_default_construction() {
    let descriptor = dynamic_descriptor();
    let scalar = DynamicScalarVariableSlot::new(
        EntityClassId(0),
        VariableId(0),
        "Task",
        "worker",
        false,
    );
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);
    let mut phase = Construction::new(None, descriptor, model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: vec![None, None],
        scalar_candidates: vec![vec![1], vec![2]],
        lists: Vec::new(),
        list_element_count: 0,
    };
    let mut solver_scope = SolverScope::new(dynamic_director(plan));

    phase.solve(&mut solver_scope);

    assert_eq!(
        solver_scope.working_solution().scalar_values,
        vec![Some(1), Some(2)]
    );
}

#[test]
fn dynamic_scalar_slot_runs_through_local_search() {
    let descriptor = dynamic_descriptor();
    let scalar = DynamicScalarVariableSlot::new(
        EntityClassId(0),
        VariableId(0),
        "Task",
        "worker",
        false,
    );
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicScalar(scalar)]);
    let config = solverforge_config::SolverConfig {
        phases: vec![
            solverforge_config::PhaseConfig::ConstructionHeuristic(
                solverforge_config::ConstructionHeuristicConfig::default(),
            ),
            solverforge_config::PhaseConfig::LocalSearch(solverforge_config::LocalSearchConfig {
                local_search_type: solverforge_config::LocalSearchType::VariableNeighborhoodDescent,
                neighborhoods: vec![solverforge_config::MoveSelectorConfig::ChangeMoveSelector(
                    solverforge_config::ChangeMoveConfig {
                        value_candidate_limit: None,
                        target: solverforge_config::VariableTargetConfig {
                            entity_class: Some("Task".to_string()),
                            variable_name: Some("worker".to_string()),
                        },
                    },
                )],
                termination: Some(solverforge_config::TerminationConfig {
                    step_count_limit: Some(4),
                    ..solverforge_config::TerminationConfig::default()
                }),
                ..solverforge_config::LocalSearchConfig::default()
            }),
        ],
        ..solverforge_config::SolverConfig::default()
    };
    let mut phases = super::build_phases(&config, &descriptor, &model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: vec![None],
        scalar_candidates: vec![vec![0, 1]],
        lists: Vec::new(),
        list_element_count: 0,
    };
    let mut solver_scope = SolverScope::new(dynamic_preference_director(plan));

    phases.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().scalar_values, vec![Some(1)]);
    assert_eq!(solver_scope.current_score().copied(), Some(SoftScore::of(0)));
}

#[test]
fn dynamic_list_slot_runs_through_local_search() {
    let descriptor = dynamic_descriptor();
    let list = DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits");
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicList(list)]);
    let config = solverforge_config::SolverConfig {
        phases: vec![solverforge_config::PhaseConfig::LocalSearch(
            solverforge_config::LocalSearchConfig {
                local_search_type: solverforge_config::LocalSearchType::VariableNeighborhoodDescent,
                neighborhoods: vec![solverforge_config::MoveSelectorConfig::ListChangeMoveSelector(
                    solverforge_config::ListChangeMoveConfig {
                        target: solverforge_config::VariableTargetConfig {
                            entity_class: Some("Vehicle".to_string()),
                            variable_name: Some("visits".to_string()),
                        },
                    },
                )],
                termination: Some(solverforge_config::TerminationConfig {
                    step_count_limit: Some(4),
                    ..solverforge_config::TerminationConfig::default()
                }),
                ..solverforge_config::LocalSearchConfig::default()
            },
        )],
        ..solverforge_config::SolverConfig::default()
    };
    let mut phases = super::build_phases(&config, &descriptor, &model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![vec![1, 0]],
        list_element_count: 2,
    };
    let mut solver_scope = SolverScope::new(dynamic_ordered_visits_director(plan));

    phases.solve(&mut solver_scope);

    assert_eq!(solver_scope.working_solution().lists, vec![vec![0, 1]]);
    assert_eq!(solver_scope.current_score().copied(), Some(SoftScore::of(0)));
}

#[test]
fn dynamic_list_slot_runs_through_default_construction() {
    let descriptor = dynamic_descriptor();
    let list = DynamicListVariableSlot::new(EntityClassId(1), VariableId(0), "Vehicle", "visits");
    let model: RuntimeModel<
        DynamicPlan,
        usize,
        DefaultCrossEntityDistanceMeter,
        DefaultCrossEntityDistanceMeter,
    > = RuntimeModel::new(vec![VariableSlot::DynamicList(list)]);
    let mut phase = Construction::new(None, descriptor, model);
    let plan = DynamicPlan {
        score: None,
        scalar_values: Vec::new(),
        scalar_candidates: Vec::new(),
        lists: vec![Vec::new(), Vec::new()],
        list_element_count: 3,
    };
    let mut solver_scope = SolverScope::new(dynamic_director(plan));

    phase.solve(&mut solver_scope);

    let assigned = solver_scope
        .working_solution()
        .lists
        .iter()
        .flat_map(|list| list.iter().copied())
        .collect::<Vec<_>>();
    assert_eq!(assigned.len(), 3);
    assert!(assigned.contains(&0));
    assert!(assigned.contains(&1));
    assert!(assigned.contains(&2));
}
