// The `auto` policy: its cost model, its switching, and that switching never
// changes the trajectory.

use std::time::Duration;

use super::super::auto::{AutoPolicy, AUTO_SWITCH_STREAK};
use super::super::EvaluationPolicy;
use super::support::{fixed, forkable_director, run, selector, starting_solution, QueenMove};
use crate::phase::localsearch::phase::{solve_local_search_with_evaluator, AdaptiveEvaluator};
use crate::phase::localsearch::{AcceptedCountForager, HillClimbingAcceptor};
use crate::scope::SolverScope;
use crate::test_utils::{create_nqueens_descriptor, NQueensSolution};

const MICRO: Duration = Duration::from_micros(1);

#[test]
fn the_cost_model_weighs_trials_against_window_overhead() {
    let policy = AutoPolicy::new(4, 10 * MICRO);
    // 32 trials of 1 µs: 32 µs sequential; one 10 µs window plus 35 trials
    // spread over 4 evaluators (8.75 µs) in parallel.
    assert_eq!(
        policy.parallel_saving_nanos(32, 32 * MICRO),
        32_000 - 18_750
    );
    // 4 trials of 1 µs cannot pay for one window.
    assert!(policy.parallel_saving_nanos(4, 4 * MICRO) < 0);
}

#[test]
fn expensive_steps_switch_to_parallel_after_a_streak() {
    let mut policy = AutoPolicy::new(4, 10 * MICRO);
    for _ in 1..AUTO_SWITCH_STREAK {
        assert!(!policy.observe_step(32, 32 * MICRO * 100));
    }
    assert!(policy.observe_step(32, 32 * MICRO * 100));
}

#[test]
fn cheap_steps_stay_sequential() {
    let mut policy = AutoPolicy::new(4, 10 * MICRO);
    for _ in 0..4 * AUTO_SWITCH_STREAK {
        assert!(!policy.observe_step(4, 4 * MICRO));
    }
}

#[test]
fn one_cheap_step_resets_the_streak() {
    let mut policy = AutoPolicy::new(4, 10 * MICRO);
    for _ in 1..AUTO_SWITCH_STREAK {
        policy.observe_step(32, 3200 * MICRO);
    }
    assert!(!policy.observe_step(4, 4 * MICRO));
    for _ in 1..AUTO_SWITCH_STREAK {
        assert!(!policy.observe_step(32, 3200 * MICRO));
    }
    assert!(policy.observe_step(32, 3200 * MICRO));
}

#[test]
fn parallel_returns_to_sequential_when_steps_get_cheap() {
    let mut policy = AutoPolicy::new(4, 10 * MICRO);
    for _ in 0..AUTO_SWITCH_STREAK {
        policy.observe_step(32, 3200 * MICRO);
    }
    assert!(policy.is_parallel());
    for _ in 1..AUTO_SWITCH_STREAK {
        assert!(policy.observe_step(4, 4 * MICRO));
    }
    assert!(!policy.observe_step(4, 4 * MICRO));
}

#[test]
fn switching_back_to_parallel_must_win_back_the_replica_rebuild() {
    let mut policy = AutoPolicy::new(4, 10 * MICRO);
    policy.record_replica_build(Duration::from_secs(1));
    // Each step saves about 2.3 ms; a streak saves about 18 ms, far less than
    // the 1 s rebuild, so the policy keeps waiting.
    for _ in 0..4 * AUTO_SWITCH_STREAK {
        assert!(!policy.observe_step(32, 3200 * MICRO));
    }
}

#[test]
fn a_single_evaluator_never_goes_parallel() {
    let mut policy = AutoPolicy::new(1, Duration::ZERO);
    for _ in 0..4 * AUTO_SWITCH_STREAK {
        assert!(!policy.observe_step(32, Duration::from_secs(1)));
    }
}

/// Runs 25 hill-climbing steps with the `auto` evaluator under `policy` and
/// returns the final rows and whether the last step ran parallel windows.
fn run_auto(policy: AutoPolicy) -> (Vec<Option<i64>>, bool) {
    let descriptor = create_nqueens_descriptor();
    let mut solver_scope =
        SolverScope::new(forkable_director!(starting_solution(), &descriptor)).with_seed(7);
    solver_scope.start_solving();
    let mut source = selector();
    let mut acceptor = HillClimbingAcceptor::new();
    let mut forager = AcceptedCountForager::new(4, false);
    let mut evaluator = AdaptiveEvaluator::<NQueensSolution, _, QueenMove>::with_policy(4, policy);
    solve_local_search_with_evaluator(
        &mut source,
        &mut (),
        &mut acceptor,
        &mut forager,
        Some(25),
        &mut evaluator,
        &mut solver_scope,
    );
    let rows = solver_scope
        .working_solution()
        .queens
        .iter()
        .map(|queen| queen.row)
        .collect();
    (rows, evaluator.ran_parallel())
}

#[test]
fn auto_goes_parallel_on_free_windows_and_keeps_the_sequential_trajectory() {
    let sequential = run(fixed(1), AcceptedCountForager::new(4, false), 25, None);
    let (rows, parallel) = run_auto(AutoPolicy::new(4, Duration::ZERO));
    assert!(parallel, "free windows make every step favour parallel");
    assert_eq!(rows, sequential.rows);
}

#[test]
fn auto_stays_sequential_on_costly_windows() {
    let sequential = run(fixed(1), AcceptedCountForager::new(4, false), 25, None);
    let (rows, parallel) = run_auto(AutoPolicy::new(4, Duration::from_secs(1)));
    assert!(!parallel);
    assert_eq!(rows, sequential.rows);
}

#[test]
fn auto_with_a_calibrated_policy_keeps_the_sequential_trajectory() {
    let sequential = run(fixed(1), AcceptedCountForager::new(4, false), 25, None);
    let auto = run(
        EvaluationPolicy::Auto(4),
        AcceptedCountForager::new(4, false),
        25,
        None,
    );
    assert_eq!(auto.rows, sequential.rows);
    assert_eq!(auto.score, sequential.score);
    assert_eq!(auto.moves_evaluated, sequential.moves_evaluated);
}
