use super::*;

#[test]
fn test_limited_neighborhood_parsing() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"

        [[phases.move_selector.selectors]]
        type = "limited_neighborhood"
        selected_count_limit = 500

        [phases.move_selector.selectors.selector]
        type = "sublist_change_move_selector"
        min_sublist_size = 1
        max_sublist_size = 3
        entity_class = "Route"
        variable_name = "visits"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    assert_eq!(config.phases.len(), 1);

    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(union.selectors.len(), 1);

    let MoveSelectorConfig::LimitedNeighborhood(limit) = &union.selectors[0] else {
        panic!("union child should be a limited neighborhood");
    };
    assert_eq!(limit.selected_count_limit, 500);

    let MoveSelectorConfig::SublistChangeMoveSelector(sublist) = limit.selector.as_ref() else {
        panic!("limited neighborhood child should be sublist_change");
    };
    assert_eq!(sublist.min_sublist_size, 1);
    assert_eq!(sublist.max_sublist_size, 3);
    assert_eq!(sublist.target.entity_class.as_deref(), Some("Route"));
    assert_eq!(sublist.target.variable_name.as_deref(), Some("visits"));
}

#[test]
fn test_union_selection_order_defaults_to_sequential() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"

        [[phases.move_selector.selectors]]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(
        union.selection_order,
        crate::move_selector::UnionSelectionOrder::Sequential
    );
}

#[test]
fn test_union_selection_order_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"
        selection_order = "round_robin"

        [[phases.move_selector.selectors]]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(
        union.selection_order,
        crate::move_selector::UnionSelectionOrder::RoundRobin
    );
}

#[test]
fn test_union_rotating_round_robin_selection_order_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"
        selection_order = "rotating_round_robin"

        [[phases.move_selector.selectors]]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(
        union.selection_order,
        crate::move_selector::UnionSelectionOrder::RotatingRoundRobin
    );
}

#[test]
fn test_union_stratified_selection_order_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"
        selection_order = "stratified_random"

        [[phases.move_selector.selectors]]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(
        union.selection_order,
        crate::move_selector::UnionSelectionOrder::StratifiedRandom
    );
}

#[test]
fn test_simulated_annealing_level_aware_config_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.acceptor]
        type = "simulated_annealing"
        level_temperatures = [100.0, 500.0]
        decay_rate = 0.999
        hill_climbing_temperature = 0.000001
        hard_regression_policy = "never_accept_hard_regression"

        [phases.acceptor.calibration]
        sample_size = 64
        target_acceptance_probability = 0.75
        fallback_temperature = 2.0
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(AcceptorConfig::SimulatedAnnealing(acceptor)) = &local_search.acceptor else {
        panic!("acceptor should be simulated annealing");
    };

    assert_eq!(acceptor.level_temperatures, Some(vec![100.0, 500.0]));
    assert_eq!(acceptor.decay_rate, Some(0.999));
    assert_eq!(acceptor.hill_climbing_temperature, Some(0.000001));
    assert_eq!(
        acceptor.hard_regression_policy,
        Some(HardRegressionPolicyConfig::NeverAcceptHardRegression)
    );
    let calibration = acceptor
        .calibration
        .as_ref()
        .expect("calibration should round trip");
    assert_eq!(calibration.sample_size, Some(64));
    assert_eq!(calibration.target_acceptance_probability, Some(0.75));
    assert_eq!(calibration.fallback_temperature, Some(2.0));
}

#[test]
fn test_ruin_recreate_defaults_to_first_fit() {
    let config = RuinRecreateMoveSelectorConfig::default();

    assert_eq!(
        RecreateHeuristicType::default(),
        RecreateHeuristicType::FirstFit
    );
    assert_eq!(
        config.recreate_heuristic_type,
        RecreateHeuristicType::FirstFit
    );
}

#[test]
fn test_round_robin_union_with_limited_neighborhood_parsing() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"
        selection_order = "round_robin"

        [[phases.move_selector.selectors]]
        type = "limited_neighborhood"
        selected_count_limit = 3

        [phases.move_selector.selectors.selector]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(
        union.selection_order,
        crate::move_selector::UnionSelectionOrder::RoundRobin
    );
    assert!(matches!(
        union.selectors[0],
        MoveSelectorConfig::LimitedNeighborhood(_)
    ));
}
