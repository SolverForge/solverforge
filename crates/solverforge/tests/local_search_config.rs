//! Local search settings that only show in a solve: the recreate heuristic of
//! ruin-and-recreate moves.

use solverforge::prelude::*;
use solverforge::SolverConfig;

#[path = "local_search_config/domain/mod.rs"]
mod domain;

use domain::Plan;

fn solve(plan: Plan, toml: &str) -> Plan {
    plan.solve_with_config(SolverConfig::from_toml_str(toml).expect("the test config parses"))
        .expect("solves")
}

/// First fit construction, then hill climbing over ruin-and-recreate moves
/// that rebuild with `recreate`.
fn ruin_recreate(recreate: &str) -> String {
    format!(
        r#"random_seed = 7

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases]]
type = "local_search"
[phases.termination]
step_count_limit = 40
[phases.acceptor]
type = "hill_climbing"
[phases.move_selector]
type = "ruin_recreate_move_selector"
min_ruin_count = 1
max_ruin_count = 2
recreate_heuristic_type = "{recreate}"
"#
    )
}

#[test]
fn cheapest_insertion_recreates_on_the_cheapest_free_worker() {
    // Two tasks, workers costing 3, 2, 1, sharing a worker costing 10. First
    // fit construction puts both tasks on worker 0 (cost 16); cheapest
    // insertion rebuilds on the cheapest free workers, reaching cost 3.
    let solved = solve(
        Plan::new(&[1, 1], &[3, 2, 1]),
        &ruin_recreate("cheapest_insertion"),
    );
    assert_eq!(solved.score, Some(SoftScore::of(-3)));
    let mut workers = solved.workers_by_task();
    workers.sort();
    assert_eq!(workers, [Some(1), Some(2)]);
}

#[test]
fn first_fit_recreate_puts_ruined_tasks_back_on_the_first_worker() {
    // Every rebuild lands on worker 0 again, so no move improves on
    // construction.
    let solved = solve(Plan::new(&[1, 1], &[3, 2, 1]), &ruin_recreate("first_fit"));
    assert_eq!(solved.score, Some(SoftScore::of(-16)));
    assert_eq!(solved.workers_by_task(), [Some(0), Some(0)]);
}
