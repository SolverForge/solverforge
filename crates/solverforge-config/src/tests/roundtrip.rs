use super::*;

#[test]
fn test_custom_phase_name_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "custom"
        name = "nurse_search"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::Custom(custom) = &reparsed.phases[0] else {
        panic!("phase should be custom");
    };

    assert_eq!(custom.name, "nurse_search");
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
    assert!(!selector.require_hard_improvement);
    assert!(selector.include_soft_matches);
}

#[test]
fn test_compound_conflict_repair_selector_roundtrip() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "compound_conflict_repair_move_selector"
        constraints = ["minimumRest"]
        max_matches_per_step = 2
        max_repairs_per_match = 3
        max_moves_per_step = 4
        include_soft_matches = true
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::CompoundConflictRepairMoveSelector(selector)) =
        &local_search.move_selector
    else {
        panic!("local search should have compound conflict repair selector");
    };

    assert_eq!(selector.constraints, ["minimumRest"]);
    assert_eq!(selector.max_matches_per_step, 2);
    assert_eq!(selector.max_repairs_per_match, 3);
    assert_eq!(selector.max_moves_per_step, 4);
    assert!(selector.require_hard_improvement);
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
        require_hard_improvement = true

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
    assert!(cartesian.require_hard_improvement);
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
