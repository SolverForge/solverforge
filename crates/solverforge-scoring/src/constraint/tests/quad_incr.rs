//! Unit tests for IncrementalQuadConstraint.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::IncrementalQuadConstraint;
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
fn test_quad_constraint_evaluate() {
    let constraint = IncrementalQuadConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
        |_s: &Solution, _a: usize, _b: usize, _c: usize, _d: usize| SimpleScore::of(1),
        false,
    );

    let solution = Solution {
        tasks: vec![
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 2 },
        ],
    };

    // One quadruple on team 1: (0, 1, 2, 3) = -1 penalty
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-1));
}

#[test]
fn test_quad_constraint_multiple_quads() {
    let constraint = IncrementalQuadConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
        |_s: &Solution, _a: usize, _b: usize, _c: usize, _d: usize| SimpleScore::of(1),
        false,
    );

    let solution = Solution {
        tasks: vec![
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
            Task { team: 1 },
        ],
    };

    // Five tasks on same team = C(5,4) = 5 quadruples
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-5));
}

#[test]
fn test_quad_constraint_incremental() {
    let mut constraint = IncrementalQuadConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
        |_s: &Solution, _a: usize, _b: usize, _c: usize, _d: usize| SimpleScore::of(1),
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

    // Initialize with 4 tasks on same team = 1 quadruple
    let total = constraint.initialize(&solution);
    assert_eq!(total, SimpleScore::of(-1));

    // Retract one task
    let delta = constraint.on_retract(&solution, 0, 0);
    // Removes the quadruple = +1
    assert_eq!(delta, SimpleScore::of(1));

    // Re-insert the task
    let delta = constraint.on_insert(&solution, 0, 0);
    // Re-adds the quadruple = -1
    assert_eq!(delta, SimpleScore::of(-1));
}

#[test]
fn test_quad_constraint_reward() {
    let constraint = IncrementalQuadConstraint::new(
        ConstraintRef::new("", "Team bonus"),
        ImpactType::Reward,
        |s: &Solution| s.tasks.as_slice(),
        |t: &Task| t.team,
        |_s: &Solution, _a: &Task, _b: &Task, _c: &Task, _d: &Task| true,
        |_s: &Solution, _a: usize, _b: usize, _c: usize, _d: usize| SimpleScore::of(5),
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

    // One quadruple = +5 reward
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(5));
}
