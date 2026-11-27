// Remaining time of a solver scope: the solve's time limit, an inherited
// deadline, and partition bounds.

use std::time::Duration;

use solverforge_config::TerminationConfig;

use crate::scope::{PartitionBound, SolverScope};
use crate::test_utils::create_simple_nqueens_director;

#[test]
fn remaining_time_follows_the_solve_time_limit() {
    let mut scope = SolverScope::new(create_simple_nqueens_director(2));
    assert_eq!(
        scope.remaining_time(),
        None,
        "no time limit, no remaining time"
    );

    scope.set_time_limit(Duration::from_secs(30));
    assert_eq!(
        scope.remaining_time(),
        Some(Duration::from_secs(30)),
        "before the solve starts the whole limit remains"
    );

    scope.start_solving();
    let remaining = scope.remaining_time().expect("a time limit applies");
    assert!(remaining <= Duration::from_secs(30));
    assert!(remaining > Duration::from_secs(29));

    scope.set_time_limit(Duration::ZERO);
    assert_eq!(scope.remaining_time(), Some(Duration::ZERO));
}

#[test]
fn remaining_time_in_a_partition_counts_the_inherited_deadline() {
    let mut parent = SolverScope::new(create_simple_nqueens_director(4));
    parent.set_time_limit(Duration::from_secs(30));
    parent.start_solving();
    let budget = parent.child_phase_budget();
    let child = parent
        .child_config(Some(&budget))
        .build_scope(create_simple_nqueens_director(4), 7);

    let remaining = child
        .remaining_time()
        .expect("the solve's deadline applies");
    assert!(remaining <= Duration::from_secs(30));
    assert!(remaining > Duration::from_secs(29));
}

#[test]
fn remaining_time_counts_the_partition_bound_and_its_parents() {
    let outer_config = TerminationConfig {
        seconds_spent_limit: Some(5),
        ..TerminationConfig::default()
    };
    let inner_config = TerminationConfig {
        seconds_spent_limit: Some(60),
        ..TerminationConfig::default()
    };
    let outer = PartitionBound::new(None, Some(&outer_config)).expect("a time bound");
    let inner = PartitionBound::new(Some(&outer), Some(&inner_config)).expect("a time bound");
    let parent = SolverScope::new(create_simple_nqueens_director(4));
    let budget = parent.child_phase_budget();
    let child = parent
        .child_config(Some(&budget))
        .with_partition_bound(Some(&inner))
        .build_scope(create_simple_nqueens_director(4), 7);

    let remaining = child.remaining_time().expect("the enclosing bound applies");
    assert!(
        remaining <= Duration::from_secs(5),
        "the outer 5 s bound ends first"
    );
    assert!(remaining > Duration::from_secs(4));
}
