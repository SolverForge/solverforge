// GroupedUniConstraint tests

use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::grouped::GroupedUniConstraint;
use crate::stream::collector::count;

#[derive(Clone)]
struct GroupedShift {
    employee_id: usize,
}

#[derive(Clone)]
struct GroupedSolution {
    shifts: Vec<GroupedShift>,
}

#[test]
fn test_grouped_constraint_evaluate() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        |s: &GroupedSolution| &s.shifts,
        |shift: &GroupedShift| shift.employee_id,
        count::<GroupedShift>(),
        |count: &usize| SimpleScore::of((*count * *count) as i64),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    // Employee 1: 3 shifts -> 9
    // Employee 2: 1 shift -> 1
    // Total penalty: -10
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(-10));
}

#[test]
fn test_grouped_constraint_incremental() {
    let mut constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        |s: &GroupedSolution| &s.shifts,
        |shift: &GroupedShift| shift.employee_id,
        count::<GroupedShift>(),
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    // Initialize
    let total = constraint.initialize(&solution);
    // Employee 1: 2 shifts -> -2
    // Employee 2: 1 shift -> -1
    // Total: -3
    assert_eq!(total, SimpleScore::of(-3));

    // Retract shift at index 0 (employee 1)
    let delta = constraint.on_retract(&solution, 0, 0);
    // Employee 1 now has 1 shift -> score goes from -2 to -1, delta = +1
    assert_eq!(delta, SimpleScore::of(1));

    // Insert shift at index 0 (employee 1)
    let delta = constraint.on_insert(&solution, 0, 0);
    // Employee 1 now has 2 shifts -> score goes from -1 to -2, delta = -1
    assert_eq!(delta, SimpleScore::of(-1));
}

#[test]
fn test_grouped_constraint_reward() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Collaboration"),
        ImpactType::Reward,
        |s: &GroupedSolution| &s.shifts,
        |shift: &GroupedShift| shift.employee_id,
        count::<GroupedShift>(),
        |count: &usize| SimpleScore::of(*count as i64),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
        ],
    };

    // 2 shifts in one group -> reward of +2
    assert_eq!(constraint.evaluate(&solution), SimpleScore::of(2));
}
