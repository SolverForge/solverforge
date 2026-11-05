// The partition bound: a partitioned phase's limits end its partitions'
// solves without terminating the solve, and chain into nested partitions.

use solverforge_config::TerminationConfig;

use crate::manager::SolverTerminalReason;
use crate::scope::{PartitionBound, PendingControl, SolverScope};
use crate::test_utils::create_simple_nqueens_director;

fn steps(limit: u64) -> TerminationConfig {
    TerminationConfig {
        step_count_limit: Some(limit),
        ..TerminationConfig::default()
    }
}

#[test]
fn a_reached_bound_ends_the_partition_without_terminating_it() {
    let parent = SolverScope::new(create_simple_nqueens_director(4));
    let bound = PartitionBound::new(None, Some(&steps(2))).expect("a step limit bounds");
    let budget = parent.child_phase_budget();
    let mut child = parent
        .child_config(Some(&budget))
        .with_partition_bound(Some(&bound))
        .build_scope(create_simple_nqueens_director(4), 1);

    child.increment_step_count();
    assert!(!child.should_terminate());
    child.increment_step_count();
    assert!(child.should_terminate());
    assert_eq!(
        child.pending_control(),
        PendingControl::ConfigTerminationRequested
    );
    assert!(!child.solve_limit_reached());
    assert_eq!(child.terminal_reason(), SolverTerminalReason::Completed);
}

#[test]
fn a_nested_bound_counts_against_the_enclosing_one() {
    let outer = PartitionBound::new(None, Some(&steps(3))).expect("a step limit bounds");
    let parent = SolverScope::new(create_simple_nqueens_director(4));
    let budget = parent.child_phase_budget();
    let partition = parent
        .child_config(Some(&budget))
        .with_partition_bound(Some(&outer))
        .build_scope(create_simple_nqueens_director(4), 1);

    let inner = PartitionBound::new(partition.partition_bound(), Some(&steps(10)))
        .expect("a step limit bounds");
    let inner_budget = partition.child_phase_budget();
    let mut nested = partition
        .child_config(Some(&inner_budget))
        .with_partition_bound(Some(&inner))
        .build_scope(create_simple_nqueens_director(4), 2);

    for _ in 0..3 {
        assert!(!nested.should_terminate());
        nested.increment_step_count();
    }
    assert!(
        nested.should_terminate(),
        "the enclosing phase's 3 steps are spent"
    );
    assert!(outer.reached());
    assert!(!PartitionBound::new(None, Some(&steps(10)))
        .unwrap()
        .reached());
}

#[test]
fn only_time_and_step_limits_make_a_bound() {
    assert!(PartitionBound::new(None, None).is_none());
    assert!(PartitionBound::new(None, Some(&TerminationConfig::default())).is_none());
    let bound = PartitionBound::new(
        None,
        Some(&TerminationConfig {
            seconds_spent_limit: Some(0),
            ..TerminationConfig::default()
        }),
    )
    .expect("a time limit bounds");
    assert!(bound.reached(), "a zero time limit is reached at once");
}
