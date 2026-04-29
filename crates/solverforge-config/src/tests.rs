// Tests for solver configuration.

use super::*;

#[test]
fn test_toml_parsing() {
    let toml = r#"
        environment_mode = "reproducible"
        random_seed = 42

        [termination]
        seconds_spent_limit = 30

        [[phases]]
        type = "construction_heuristic"
        construction_heuristic_type = "first_fit_decreasing"

        [[phases]]
        type = "local_search"
        [phases.acceptor]
        type = "late_acceptance"
        late_acceptance_size = 400
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    assert_eq!(config.environment_mode, EnvironmentMode::Reproducible);
    assert_eq!(config.random_seed, Some(42));
    assert_eq!(config.termination.unwrap().seconds_spent_limit, Some(30));
    assert_eq!(config.phases.len(), 2);
}

#[test]
fn test_yaml_parsing() {
    let yaml = r#"
        environment_mode: reproducible
        random_seed: 42
        termination:
          seconds_spent_limit: 30
        phases:
          - type: construction_heuristic
            construction_heuristic_type: first_fit_decreasing
          - type: local_search
            acceptor:
              type: late_acceptance
              late_acceptance_size: 400
    "#;

    let config = SolverConfig::from_yaml_str(yaml).unwrap();
    assert_eq!(config.environment_mode, EnvironmentMode::Reproducible);
    assert_eq!(config.random_seed, Some(42));
}

#[test]
fn test_builder() {
    let config = SolverConfig::new()
        .with_random_seed(123)
        .with_termination_seconds(60)
        .with_phase(PhaseConfig::ConstructionHeuristic(
            ConstructionHeuristicConfig::default(),
        ))
        .with_phase(PhaseConfig::LocalSearch(LocalSearchConfig::default()));

    assert_eq!(config.random_seed, Some(123));
    assert_eq!(config.phases.len(), 2);
}

#[test]
fn test_target_and_vnd_parsing() {
    let toml = r#"
        [[phases]]
        type = "construction_heuristic"
        construction_heuristic_type = "first_fit"
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases]]
        type = "vnd"

        [[phases.neighborhoods]]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.neighborhoods]]
        type = "list_change_move_selector"
        entity_class = "Vehicle"
        variable_name = "visits"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    assert_eq!(config.phases.len(), 2);

    let PhaseConfig::ConstructionHeuristic(construction) = &config.phases[0] else {
        panic!("first phase should be construction");
    };
    assert_eq!(
        construction.construction_obligation,
        ConstructionObligation::PreserveUnassigned
    );
    assert_eq!(construction.target.entity_class.as_deref(), Some("Shift"));
    assert_eq!(
        construction.target.variable_name.as_deref(),
        Some("employee_id")
    );

    let PhaseConfig::Vnd(vnd) = &config.phases[1] else {
        panic!("second phase should be vnd");
    };
    assert_eq!(vnd.neighborhoods.len(), 2);

    let MoveSelectorConfig::ChangeMoveSelector(change) = &vnd.neighborhoods[0] else {
        panic!("first neighborhood should be change selector");
    };
    assert_eq!(change.target.entity_class.as_deref(), Some("Shift"));
    assert_eq!(change.target.variable_name.as_deref(), Some("employee_id"));

    let MoveSelectorConfig::ListChangeMoveSelector(list_change) = &vnd.neighborhoods[1] else {
        panic!("second neighborhood should be list change selector");
    };
    assert_eq!(list_change.target.entity_class.as_deref(), Some("Vehicle"));
    assert_eq!(list_change.target.variable_name.as_deref(), Some("visits"));
}

#[test]
fn construction_obligation_parses_and_roundtrips() {
    let toml = r#"
        [[phases]]
        type = "construction_heuristic"
        construction_heuristic_type = "cheapest_insertion"
        construction_obligation = "assign_when_candidate_exists"
        entity_class = "Shift"
        variable_name = "employee_id"
        value_candidate_limit = 32
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let PhaseConfig::ConstructionHeuristic(construction) = &config.phases[0] else {
        panic!("phase should be construction");
    };
    assert_eq!(
        construction.construction_obligation,
        ConstructionObligation::AssignWhenCandidateExists
    );
    assert_eq!(
        construction.construction_heuristic_type,
        ConstructionHeuristicType::CheapestInsertion
    );
    assert_eq!(construction.target.entity_class.as_deref(), Some("Shift"));
    assert_eq!(
        construction.target.variable_name.as_deref(),
        Some("employee_id")
    );
    assert_eq!(construction.value_candidate_limit, Some(32));

    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::ConstructionHeuristic(reparsed_construction) = &reparsed.phases[0] else {
        panic!("reparsed phase should be construction");
    };
    assert_eq!(
        reparsed_construction.construction_obligation,
        ConstructionObligation::AssignWhenCandidateExists
    );
}

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

#[test]
fn test_conflict_repair_selector_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "conflict_repair_move_selector"
        constraints = ["minimumRest", "furnaceOverlap"]
        max_matches_per_step = 4
        max_repairs_per_match = 8
        max_moves_per_step = 32
        include_soft_matches = true
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::ConflictRepairMoveSelector(selector)) =
        &local_search.move_selector
    else {
        panic!("local search should have conflict repair selector");
    };

    assert_eq!(selector.constraints, ["minimumRest", "furnaceOverlap"]);
    assert_eq!(selector.max_matches_per_step, 4);
    assert_eq!(selector.max_repairs_per_match, 8);
    assert_eq!(selector.max_moves_per_step, 32);
    assert!(selector.include_soft_matches);
}

#[test]
fn test_new_scalar_selector_variants_and_cartesian_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"

        [[phases.move_selector.selectors]]
        type = "nearby_change_move_selector"
        max_nearby = 7
        value_candidate_limit = 3
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.move_selector.selectors]]
        type = "nearby_swap_move_selector"
        max_nearby = 5
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.move_selector.selectors]]
        type = "pillar_change_move_selector"
        minimum_sub_pillar_size = 2
        maximum_sub_pillar_size = 4
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.move_selector.selectors]]
        type = "pillar_swap_move_selector"
        minimum_sub_pillar_size = 2
        maximum_sub_pillar_size = 3
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.move_selector.selectors]]
        type = "ruin_recreate_move_selector"
        min_ruin_count = 2
        max_ruin_count = 6
        moves_per_step = 9
        recreate_heuristic_type = "first_fit"
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.move_selector.selectors]]
        type = "cartesian_product_move_selector"

        [[phases.move_selector.selectors.selectors]]
        type = "change_move_selector"
        entity_class = "Shift"
        variable_name = "employee_id"

        [[phases.move_selector.selectors.selectors]]
        type = "list_change_move_selector"
        entity_class = "Route"
        variable_name = "visits"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::UnionMoveSelector(union)) = &local_search.move_selector else {
        panic!("local search should have a union move selector");
    };
    assert_eq!(union.selectors.len(), 6);

    let MoveSelectorConfig::NearbyChangeMoveSelector(nearby_change) = &union.selectors[0] else {
        panic!("first selector should be nearby_change");
    };
    assert_eq!(nearby_change.max_nearby, 7);
    assert_eq!(nearby_change.value_candidate_limit, Some(3));

    let MoveSelectorConfig::NearbySwapMoveSelector(nearby_swap) = &union.selectors[1] else {
        panic!("second selector should be nearby_swap");
    };
    assert_eq!(nearby_swap.max_nearby, 5);

    let MoveSelectorConfig::PillarChangeMoveSelector(pillar_change) = &union.selectors[2] else {
        panic!("third selector should be pillar_change");
    };
    assert_eq!(pillar_change.minimum_sub_pillar_size, 2);
    assert_eq!(pillar_change.maximum_sub_pillar_size, 4);

    let MoveSelectorConfig::PillarSwapMoveSelector(pillar_swap) = &union.selectors[3] else {
        panic!("fourth selector should be pillar_swap");
    };
    assert_eq!(pillar_swap.minimum_sub_pillar_size, 2);
    assert_eq!(pillar_swap.maximum_sub_pillar_size, 3);

    let MoveSelectorConfig::RuinRecreateMoveSelector(ruin_recreate) = &union.selectors[4] else {
        panic!("fifth selector should be ruin_recreate");
    };
    assert_eq!(ruin_recreate.min_ruin_count, 2);
    assert_eq!(ruin_recreate.max_ruin_count, 6);
    assert_eq!(ruin_recreate.moves_per_step, Some(9));
    assert_eq!(
        ruin_recreate.recreate_heuristic_type,
        RecreateHeuristicType::FirstFit
    );

    let MoveSelectorConfig::CartesianProductMoveSelector(cartesian) = &union.selectors[5] else {
        panic!("sixth selector should be cartesian_product");
    };
    assert_eq!(cartesian.selectors.len(), 2);
}

#[test]
fn test_tagged_forager_variants_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.forager]
        type = "accepted_count"
        limit = 9
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(ForagerConfig::AcceptedCount(accepted_count)) = &local_search.forager else {
        panic!("forager should be accepted_count");
    };
    assert_eq!(accepted_count.limit, Some(9));

    let improving_toml = r#"
        [[phases]]
        type = "local_search"

        [phases.forager]
        type = "first_best_score_improving"
    "#;

    let config = SolverConfig::from_toml_str(improving_toml).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    assert!(matches!(
        local_search.forager,
        Some(ForagerConfig::FirstBestScoreImproving)
    ));
}
