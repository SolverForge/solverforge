// Unit tests for IncrementalTriConstraint.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::IncrementalTriConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Task {
    team: u32,
}

#[derive(Clone)]
struct Solution {
    tasks: Vec<Task>,
}

fn tasks(s: &Solution) -> &[Task] {
    s.tasks.as_slice()
}

#[test]
fn test_tri_constraint_evaluate() {
    let constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        source(
            tasks as fn(&Solution) -> &[Task],
            ChangeSource::Descriptor(0),
        ),
        |_s: &Solution, t: &Task, _idx: usize| t.team,
        |_s: &Solution,
         _a: &Task,
         _b: &Task,
         _c: &Task,
         _a_idx: usize,
         _b_idx: usize,
         _c_idx: usize| true,
        |_s: &Solution, _entities: &[Task], _a_idx: usize, _b_idx: usize, _c_idx: usize| {
            SoftScore::of(1)
        },
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
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-1));
}

#[test]
fn test_tri_constraint_multiple_triples() {
    let constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        source(
            tasks as fn(&Solution) -> &[Task],
            ChangeSource::Descriptor(0),
        ),
        |_s: &Solution, t: &Task, _idx: usize| t.team,
        |_s: &Solution,
         _a: &Task,
         _b: &Task,
         _c: &Task,
         _a_idx: usize,
         _b_idx: usize,
         _c_idx: usize| true,
        |_s: &Solution, _entities: &[Task], _a_idx: usize, _b_idx: usize, _c_idx: usize| {
            SoftScore::of(1)
        },
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
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-4));
}

#[test]
fn test_tri_constraint_incremental() {
    let mut constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Cluster"),
        ImpactType::Penalty,
        source(
            tasks as fn(&Solution) -> &[Task],
            ChangeSource::Descriptor(0),
        ),
        |_s: &Solution, t: &Task, _idx: usize| t.team,
        |_s: &Solution,
         _a: &Task,
         _b: &Task,
         _c: &Task,
         _a_idx: usize,
         _b_idx: usize,
         _c_idx: usize| true,
        |_s: &Solution, _entities: &[Task], _a_idx: usize, _b_idx: usize, _c_idx: usize| {
            SoftScore::of(1)
        },
        false,
    );

    let solution = Solution {
        tasks: vec![Task { team: 1 }, Task { team: 1 }, Task { team: 1 }],
    };

    // Initialize with 3 tasks on same team = 1 triple
    let total = constraint.initialize(&solution);
    assert_eq!(total, SoftScore::of(-1));

    // Retract one task
    let delta = constraint.on_retract(&solution, 0, 0);
    // Removes the triple = +1
    assert_eq!(delta, SoftScore::of(1));

    // Re-insert the task
    let delta = constraint.on_insert(&solution, 0, 0);
    // Re-adds the triple = -1
    assert_eq!(delta, SoftScore::of(-1));
}

#[test]
fn tri_filter_receives_source_indexes() {
    let mut constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Indexed cluster"),
        ImpactType::Penalty,
        source(
            tasks as fn(&Solution) -> &[Task],
            ChangeSource::Descriptor(0),
        ),
        |_s: &Solution, t: &Task, _idx: usize| t.team,
        |_s: &Solution,
         _a: &Task,
         _b: &Task,
         _c: &Task,
         a_idx: usize,
         b_idx: usize,
         c_idx: usize| { (a_idx, b_idx, c_idx) == (1, 2, 3) },
        |_s: &Solution, _entities: &[Task], _a_idx: usize, _b_idx: usize, _c_idx: usize| {
            SoftScore::of(1)
        },
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

    assert_eq!(constraint.match_count(&solution), 1);
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-1));
    assert_eq!(constraint.initialize(&solution), SoftScore::of(-1));
}

#[test]
fn test_tri_constraint_reward() {
    let constraint = IncrementalTriConstraint::new(
        ConstraintRef::new("", "Team bonus"),
        ImpactType::Reward,
        source(
            tasks as fn(&Solution) -> &[Task],
            ChangeSource::Descriptor(0),
        ),
        |_s: &Solution, t: &Task, _idx: usize| t.team,
        |_s: &Solution,
         _a: &Task,
         _b: &Task,
         _c: &Task,
         _a_idx: usize,
         _b_idx: usize,
         _c_idx: usize| true,
        |_s: &Solution, _entities: &[Task], _a_idx: usize, _b_idx: usize, _c_idx: usize| {
            SoftScore::of(5)
        },
        false,
    );

    let solution = Solution {
        tasks: vec![Task { team: 1 }, Task { team: 1 }, Task { team: 1 }],
    };

    // One triple = +5 reward
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(5));
}
