//! Bounded Clarke-Wright savings through the stock `domain = "cvrp"` profile:
//! the configured bound reaches the profile's neighbour hook, a bound covering
//! every pair reproduces unbounded construction, and a small bound still
//! assigns every customer.

use solverforge::SolverConfig;

#[path = "list_clarke_wright_publication/domain/mod.rs"]
mod domain;

use domain::{build_plan, PublicationPlan};

fn construct(customers: usize, savings_neighbors: Option<usize>) -> Vec<Vec<usize>> {
    let bound = savings_neighbors
        .map(|k| format!("savings_neighbors = {k}\n"))
        .unwrap_or_default();
    let config = SolverConfig::from_toml_str(&format!(
        "[[phases]]\ntype = \"construction_heuristic\"\n\
         construction_heuristic_type = \"list_clarke_wright\"\n{bound}"
    ))
    .expect("Clarke-Wright config parses");
    let plan: PublicationPlan = build_plan(customers, 1);
    plan.solve_with_config(config)
        .expect("construction solves")
        .routes
        .into_iter()
        .map(|route| route.visits)
        .collect()
}

#[test]
fn a_bound_covering_every_pair_reproduces_unbounded_construction() {
    let unbounded = construct(24, None);
    assert_eq!(construct(24, Some(23)), unbounded);
    assert_eq!(construct(24, Some(100)), unbounded);
}

#[test]
fn a_small_bound_still_assigns_every_customer() {
    let routes = construct(24, Some(3));
    let mut assigned: Vec<usize> = routes.concat();
    assigned.sort_unstable();
    let mut expected: Vec<usize> = build_plan(24, 1).customer_values;
    expected.sort_unstable();
    assert_eq!(assigned, expected);
}
