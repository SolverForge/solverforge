//! Unit tests for IncrementalTriConstraint.

use super::tri_incremental::IncrementalTriConstraint;
use crate::api::constraint_set::IncrementalConstraint;
use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Task {
    team: u32,
}

#[derive(Clone)]
struct Solution {
    tasks: Vec<Task>,
}

#[test]
fn test_tri_constraint_evaluate() {
    let constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_a: &Task, _b: &Task, _c: &Task| true,
        |_a: &Task, _b: &Task, _c: &Task| SimpleScore::of(1),
        false,
    );

    let solution = Solution {
        tasks: vec![
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 2 },
        ],
    };

    // One triple on team 1: (0, 1, 2) = -1 penalty
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
}

#[test]
fn test_tri_constraint_multiple_triples() {
    let constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_a: &Task, _b: &Task, _c: &Task| true,
        |_a: &Task, _b: &Task, _c: &Task| SimpleScore::of(1),
        false,
    );

    let solution = Solution {
        tasks: vec![
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
        ],
    };

    // Four tasks on same team = C(4,3) = 4 triples
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-4));
}

#[test]
fn test_tri_constraint_incremental() {
    let mut constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_a: &Task, _b: &Task, _c: &Task| true,
        |_a: &Task, _b: &Task, _c: &Task| SimpleScore::of(1),
        false,
    );

    let solution = Solution {
        tasks: vec![Task { team: 1 }, Task { team: 1 }, Task { team: 1 }],
    };

    // Initialize with 3 tasks on same team = 1 triple
    let total = constraint.initialize(&solution);
    assert_eq!(total, SimpleScore::of(-1));

    // Retract one task
    let delta = constraint.on_retract(&solution, 0);
    // Removes the triple = +1
    assert_eq!(delta, SimpleScore::of(1));

    // Re-insert the task
    let delta = constraint.on_insert(&solution, 0);
    // Re-adds the triple = -1
    assert_eq!(delta, SimpleScore::of(-1));
}

#[test]
fn test_tri_constraint_reward() {
    let constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Team bonus"),
        ImpactType::Reward,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_a: &Task, _b: &Task, _c: &Task| true,
        |_a: &Task, _b: &Task, _c: &Task| SimpleScore::of(5),
        false,
    );

    let solution = Solution {
        tasks: vec![Task { team: 1 }, Task { team: 1 }, Task { team: 1 }],
    };

    // One triple = +5 reward
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(5));
}
