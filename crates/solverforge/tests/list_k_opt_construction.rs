//! `list_k_opt` construction polishes routes with the configured `k`.

use solverforge::SolverConfig;

#[path = "list_k_opt_construction/domain/mod.rs"]
mod domain;

use domain::sorting_plan;

fn polish_with_k(k: usize) -> Vec<usize> {
    let config = SolverConfig::from_toml_str(&format!(
        r#"
environment_mode = "reproducible"

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "list_k_opt"
entity_class = "Route"
variable_name = "visits"
k = {k}
"#
    ))
    .expect("the test config parses");
    sorting_plan()
        .solve_with_config(config)
        .expect("solves")
        .routes[0]
        .visits
        .clone()
}

#[test]
fn configured_k_reaches_the_list_k_opt_construction_kernel() {
    assert_eq!(polish_with_k(2), [1, 4, 5, 2, 3, 6]);
    for k in 3..=5 {
        assert_eq!(polish_with_k(k), [1, 2, 3, 4, 5, 6], "k = {k}");
    }
}
