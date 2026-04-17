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
fn test_selected_count_limit_move_selector_parsing() {
    let toml = r#"
        [[phases]]
        type = "local_search"

        [phases.move_selector]
        type = "union_move_selector"

        [[phases.move_selector.selectors]]
        type = "selected_count_limit_move_selector"
        selected_count_limit = 500

        [phases.move_selector.selectors.selector]
        type = "sub_list_change_move_selector"
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

    let MoveSelectorConfig::SelectedCountLimitMoveSelector(limit) = &union.selectors[0] else {
        panic!("union child should be a selected count limit selector");
    };
    assert_eq!(limit.selected_count_limit, 500);

    let MoveSelectorConfig::SubListChangeMoveSelector(sublist) = limit.selector.as_ref() else {
        panic!("selected count limit child should be sub_list_change");
    };
    assert_eq!(sublist.min_sublist_size, 1);
    assert_eq!(sublist.max_sublist_size, 3);
    assert_eq!(sublist.target.entity_class.as_deref(), Some("Route"));
    assert_eq!(sublist.target.variable_name.as_deref(), Some("visits"));
}
