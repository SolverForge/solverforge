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
        group_name = "shift_bundle"
        group_candidate_limit = 12
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
    assert_eq!(construction.group_name.as_deref(), Some("shift_bundle"));
    assert_eq!(construction.group_candidate_limit, Some(12));

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
fn coverage_first_fit_construction_parses_and_roundtrips() {
    let toml = r#"
        [[phases]]
        type = "construction_heuristic"
        construction_heuristic_type = "coverage_first_fit"
        construction_obligation = "assign_when_candidate_exists"
        group_name = "shift_nurse_coverage"
        value_candidate_limit = 32
        group_candidate_limit = 512
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::ConstructionHeuristic(construction) = &reparsed.phases[0] else {
        panic!("phase should be construction");
    };

    assert_eq!(
        construction.construction_heuristic_type,
        ConstructionHeuristicType::CoverageFirstFit
    );
    assert_eq!(
        construction.construction_obligation,
        ConstructionObligation::AssignWhenCandidateExists
    );
    assert_eq!(
        construction.group_name.as_deref(),
        Some("shift_nurse_coverage")
    );
    assert_eq!(construction.value_candidate_limit, Some(32));
    assert_eq!(construction.group_candidate_limit, Some(512));
}

#[test]
fn grouped_scalar_move_selector_parses_and_roundtrips() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "grouped_scalar_move_selector"
        group_name = "task_operator_assignment"
        value_candidate_limit = 24
        max_moves_per_step = 64
        require_hard_improvement = true
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::GroupedScalarMoveSelector(selector)) = &local_search.move_selector
    else {
        panic!("local search should have grouped scalar selector");
    };

    assert_eq!(selector.group_name, "task_operator_assignment");
    assert_eq!(selector.value_candidate_limit, Some(24));
    assert_eq!(selector.max_moves_per_step, Some(64));
    assert!(selector.require_hard_improvement);
}

#[test]
fn coverage_repair_move_selector_parses_and_roundtrips() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "coverage_repair_move_selector"
        group_name = "shift_nurse_coverage"
        value_candidate_limit = 24
        max_moves_per_step = 64
        require_hard_improvement = true
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let encoded = toml::to_string(&config).unwrap();
    let reparsed = SolverConfig::from_toml_str(&encoded).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &reparsed.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::CoverageRepairMoveSelector(selector)) =
        &local_search.move_selector
    else {
        panic!("local search should have coverage repair selector");
    };

    assert_eq!(selector.group_name, "shift_nurse_coverage");
    assert_eq!(selector.value_candidate_limit, Some(24));
    assert_eq!(selector.max_moves_per_step, Some(64));
    assert!(selector.require_hard_improvement);
}

#[test]
fn coverage_repair_move_selector_omits_repair_cap_when_unconfigured() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "coverage_repair_move_selector"
        group_name = "shift_nurse_coverage"
    "#;

    let config = SolverConfig::from_toml_str(toml).unwrap();
    let PhaseConfig::LocalSearch(local_search) = &config.phases[0] else {
        panic!("phase should be local_search");
    };
    let Some(MoveSelectorConfig::CoverageRepairMoveSelector(selector)) =
        &local_search.move_selector
    else {
        panic!("local search should have coverage repair selector");
    };

    assert_eq!(selector.group_name, "shift_nurse_coverage");
    assert_eq!(selector.max_moves_per_step, None);
}
