//! Tests for solver configuration.

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
