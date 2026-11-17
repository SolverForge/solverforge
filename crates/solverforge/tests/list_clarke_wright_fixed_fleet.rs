//! Clarke-Wright construction on a fixed fleet: when the savings routes
//! outnumber the vehicles, the routes the fleet can take stay committed and
//! the default construction places the remaining customers, so a fixed fleet
//! never ends construction with every customer unassigned.

use solverforge::SolverConfig;

#[path = "list_clarke_wright_fixed_fleet/domain/mod.rs"]
mod domain;

use domain::{build_fleet_plan, FleetPlan};

const CUSTOMERS: usize = 20;

fn solve(plan: FleetPlan, toml: &str) -> FleetPlan {
    let config = SolverConfig::from_toml_str(toml).expect("config parses");
    plan.solve_with_config(config).expect("solve completes")
}

fn assigned_customers(plan: &FleetPlan) -> Vec<usize> {
    let mut assigned: Vec<usize> = plan
        .routes
        .iter()
        .flat_map(|route| route.visits.iter().copied())
        .collect();
    assigned.sort_unstable();
    assigned
}

fn every_customer() -> Vec<usize> {
    (1..=CUSTOMERS).collect()
}

const DEFAULT_POLICY: &str = "environment_mode = \"reproducible\"\n\
     random_seed = 42\n\
     [termination]\n\
     step_count_limit = 20\n";

#[test]
fn default_construction_completes_a_fleet_smaller_than_the_savings_routes() {
    // Capacity 5 over 20 unit demands needs four routes; two vehicles exist.
    let solved = solve(build_fleet_plan(CUSTOMERS, 2, 5, None), DEFAULT_POLICY);
    assert_eq!(assigned_customers(&solved), every_customer());
    assert_eq!(solved.score.map(|score| score.hard()), Some(0));
}

#[test]
fn default_construction_completes_a_fixed_fleet_with_time_windows() {
    let solved = solve(build_fleet_plan(CUSTOMERS, 3, 4, Some(12)), DEFAULT_POLICY);
    assert_eq!(assigned_customers(&solved), every_customer());
}

#[test]
fn explicit_clarke_wright_keeps_the_routes_the_fleet_can_take() {
    let solved = solve(
        build_fleet_plan(CUSTOMERS, 2, 5, None),
        "environment_mode = \"reproducible\"\n\
         random_seed = 42\n\
         [[phases]]\n\
         type = \"construction_heuristic\"\n\
         construction_heuristic_type = \"list_clarke_wright\"\n\
         [[phases]]\n\
         type = \"construction_heuristic\"\n\
         construction_heuristic_type = \"list_cheapest_insertion\"\n",
    );
    assert_eq!(assigned_customers(&solved), every_customer());
    assert!(
        solved.routes.iter().all(|route| !route.visits.is_empty()),
        "both vehicles carry a committed route"
    );
}

#[test]
fn a_fleet_that_fits_the_savings_routes_is_unchanged_by_completion() {
    let solved = solve(build_fleet_plan(CUSTOMERS, 4, 5, None), DEFAULT_POLICY);
    assert_eq!(assigned_customers(&solved), every_customer());
}
