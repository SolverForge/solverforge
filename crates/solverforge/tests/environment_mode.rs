//! `environment_mode = "reproducible"` without `random_seed` solves with
//! `REPRODUCIBLE_RANDOM_SEED`, so repeated runs give the same result.

use solverforge::{SolverConfig, REPRODUCIBLE_RANDOM_SEED};

#[path = "environment_mode/domain/mod.rs"]
mod domain;

use domain::Plan;

/// Random move order and a random acceptor, bounded by steps only.
fn config(header: &str) -> SolverConfig {
    SolverConfig::from_toml_str(&format!(
        r#"{header}
[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases]]
type = "local_search"
[phases.termination]
step_count_limit = 300
[phases.acceptor]
type = "simulated_annealing"
level_temperatures = [0.5, 2.0]
decay_rate = 0.999
[phases.move_selector]
type = "change_move_selector"
selection_order = "random"
"#
    ))
    .expect("the test config parses")
}

fn assignments(header: &str) -> Vec<Option<usize>> {
    Plan::new()
        .solve_with_config(config(header))
        .expect("solves")
        .tasks
        .into_iter()
        .map(|task| task.resource_idx)
        .collect()
}

#[test]
fn the_seed_decides_the_result() {
    // Without this, the reproducibility test below could pass on a search
    // that ignores its seed.
    let first = assignments("random_seed = 1");
    assert!(
        (2..10).any(|seed| assignments(&format!("random_seed = {seed}")) != first),
        "every seed gave the same assignment"
    );
}

#[test]
fn reproducible_runs_use_the_documented_seed() {
    let seeded = assignments(&format!("random_seed = {REPRODUCIBLE_RANDOM_SEED}"));
    for _ in 0..3 {
        assert_eq!(assignments("environment_mode = \"reproducible\""), seeded);
    }
    // Every asserting mode is reproducible too.
    assert_eq!(assignments("environment_mode = \"fast_assert\""), seeded);
    assert_eq!(assignments("environment_mode = \"full_assert\""), seeded);
}

#[test]
fn only_non_reproducible_draws_its_seed_from_the_operating_system() {
    let reproducible = SolverConfig::from_toml_str("environment_mode = \"reproducible\"").unwrap();
    assert_eq!(
        reproducible.resolved_random_seed(),
        Some(REPRODUCIBLE_RANDOM_SEED)
    );
    assert_eq!(SolverConfig::default().resolved_random_seed(), None);
    let seeded = SolverConfig::from_toml_str("random_seed = 9").unwrap();
    assert_eq!(seeded.resolved_random_seed(), Some(9));
}
