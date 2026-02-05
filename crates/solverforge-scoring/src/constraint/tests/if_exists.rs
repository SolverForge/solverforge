// IfExistsUniConstraint tests

use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::if_exists::{ExistenceMode, IfExistsUniConstraint};

#[derive(Clone)]
struct Task {
    _id: usize,
    assignee: Option<usize>,
}

#[derive(Clone)]
struct Worker {
    id: usize,
    available: bool,
}

#[derive(Clone)]
struct TaskSchedule {
    tasks: Vec<Task>,
    workers: Vec<Worker>,
}

#[test]
fn test_if_exists_penalizes_assigned_to_unavailable() {
    // Penalize tasks assigned to unavailable workers
    let constraint = IfExistsUniConstraint::new(
        ConstraintRef::new("", "Unavailable worker"),
        ImpactType::Penalty,
        ExistenceMode::Exists,
        |s: &TaskSchedule| s.tasks.as_slice(),
        |s: &TaskSchedule| s.workers.iter().filter(|w| !w.available).cloned().collect(),
        |t: &Task| t.assignee,
        |w: &Worker| Some(w.id),
        |_s: &TaskSchedule, t: &Task| t.assignee.is_some(),
        |_t: &Task| SimpleScore::of(1),
        false,
    );

    let schedule = TaskSchedule {
        tasks: vec![
            Task {
                _id: 0,
                assignee: Some(0),
            }, // assigned to unavailable
            Task {
                _id: 1,
                assignee: Some(1),
            }, // assigned to available
            Task {
                _id: 2,
                assignee: None,
            }, // unassigned
        ],
        workers: vec![
            Worker {
                id: 0,
                available: false,
            },
            Worker {
                id: 1,
                available: true,
            },
        ],
    };

    // Only task 0 matches
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    assert_eq!(constraint.match_count(&schedule), 1);
}

#[test]
fn test_if_not_exists_penalizes_unassigned() {
    // Penalize tasks not assigned to any available worker
    let constraint = IfExistsUniConstraint::new(
        ConstraintRef::new("", "No available worker"),
        ImpactType::Penalty,
        ExistenceMode::NotExists,
        |s: &TaskSchedule| s.tasks.as_slice(),
        |s: &TaskSchedule| s.workers.iter().filter(|w| w.available).cloned().collect(),
        |t: &Task| t.assignee,
        |w: &Worker| Some(w.id),
        |_s: &TaskSchedule, t: &Task| t.assignee.is_some(),
        |_t: &Task| SimpleScore::of(1),
        false,
    );

    let schedule = TaskSchedule {
        tasks: vec![
            Task {
                _id: 0,
                assignee: Some(0),
            }, // assigned to unavailable - no match in available
            Task {
                _id: 1,
                assignee: Some(1),
            }, // assigned to available
            Task {
                _id: 2,
                assignee: None,
            }, // unassigned - filtered out by filter_a
        ],
        workers: vec![
            Worker {
                id: 0,
                available: false,
            },
            Worker {
                id: 1,
                available: true,
            },
        ],
    };

    // Task 0 is assigned but worker 0 is not available
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
    assert_eq!(constraint.match_count(&schedule), 1);
}
