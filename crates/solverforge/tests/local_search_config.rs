//! Local search settings that only show in a solve: the recreate heuristic of
//! ruin-and-recreate moves, swap moves, and that each random part of a search
//! (simulated annealing, random tie breaks, ruin choice) follows
//! `random_seed`.

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

#[test]
fn swap_moves_exchange_two_tasks_workers() {
    // Task 0 (weight 5) on worker 1 (cost 3) and task 1 (weight 1) on
    // worker 0 (cost 1) cost 16; exchanging their workers costs 8. Only swap
    // moves are configured, so only an exchange can improve.
    let mut plan = Plan::new(&[5, 1], &[1, 3]);
    plan.tasks[0].worker = Some(1);
    plan.tasks[1].worker = Some(0);
    let solved = solve(
        plan,
        r#"random_seed = 7

[[phases]]
type = "local_search"
[phases.termination]
step_count_limit = 10
[phases.acceptor]
type = "hill_climbing"
[phases.move_selector]
type = "swap_move_selector"
"#,
    );
    assert_eq!(solved.workers_by_task(), [Some(0), Some(1)]);
    assert_eq!(solved.score, Some(SoftScore::of(-8)));
}

/// First fit construction, then local search configured by `local_search`, on
/// twelve tasks weighing 0 to 8 and four workers costing 1 to 4: many local
/// optima and equally scored moves, so a random part decides where a bounded
/// search ends.
fn stochastic(seed: u64, local_search: &str) -> Vec<Option<usize>> {
    let toml = format!(
        r#"random_seed = {seed}

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"

[[phases]]
type = "local_search"
{local_search}
"#
    );
    let weights: Vec<i64> = (0..12).map(|id| (id * 5 + 2) % 9).collect();
    solve(Plan::new(&weights, &[1, 2, 3, 4]), &toml).workers_by_task()
}

/// Repeated solves with one seed agree, and some other seed disagrees, so
/// the agreement is not a search that ignores its seed.
fn assert_seeded(local_search: &str) {
    let first = stochastic(7, local_search);
    for _ in 0..3 {
        assert_eq!(stochastic(7, local_search), first);
    }
    assert!(
        (1..10).any(|seed| stochastic(seed, local_search) != first),
        "every seed gave the same assignment"
    );
}

#[test]
fn simulated_annealing_follows_the_random_seed() {
    // Moves in original order and the first accepted one taken: only
    // annealing draws random numbers.
    assert_seeded(
        r#"score_tie_break = "first"
[phases.termination]
step_count_limit = 200
[phases.acceptor]
type = "simulated_annealing"
level_temperatures = [20.0]
decay_rate = 0.999
[phases.forager]
type = "first_accepted"
[phases.move_selector]
type = "change_move_selector"
selection_order = "original""#,
    );
}

#[test]
fn random_tie_breaks_follow_the_random_seed() {
    // Hill climbing over moves in original order: only tie breaks among
    // equally scored moves are random.
    assert_seeded(
        r#"score_tie_break = "random"
[phases.termination]
step_count_limit = 200
[phases.acceptor]
type = "hill_climbing"
[phases.forager]
type = "best_score"
[phases.move_selector]
type = "change_move_selector"
selection_order = "original""#,
    );
}

#[test]
fn ruin_choice_follows_the_random_seed() {
    // Hill climbing taking the first accepted move: only which tasks a
    // ruin-and-recreate move unassigns is random.
    assert_seeded(
        r#"score_tie_break = "first"
[phases.termination]
step_count_limit = 200
[phases.acceptor]
type = "hill_climbing"
[phases.forager]
type = "first_accepted"
[phases.move_selector]
type = "ruin_recreate_move_selector"
min_ruin_count = 1
max_ruin_count = 3
recreate_heuristic_type = "cheapest_insertion""#,
    );
}
