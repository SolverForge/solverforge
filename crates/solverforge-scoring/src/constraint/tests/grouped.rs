// GroupedUniConstraint tests

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::grouped::GroupedUniConstraint;
use crate::stream::collection_extract::{source, vec, ChangeSource};
use crate::stream::collector::{collect_vec, count, CollectedVec};
use crate::stream::filter::TrueFilter;

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
        source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |_employee_id: &usize, count: &usize| SoftScore::of((*count * *count) as i64),
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

    /* Employee 1: 3 shifts -> 9
    Employee 2: 1 shift -> 1
    Total penalty: -10
    */
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-10));
}

#[test]
fn test_grouped_constraint_incremental() {
    let mut constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Workload"),
        ImpactType::Penalty,
        source(
            vec(|s: &GroupedSolution| &s.shifts),
            ChangeSource::Descriptor(0),
        ),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
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
    /* Employee 1: 2 shifts -> -2
    Employee 2: 1 shift -> -1
    Total: -3
    */
    assert_eq!(total, SoftScore::of(-3));

    // Retract shift at index 0 (employee 1)
    let delta = constraint.on_retract(&solution, 0, 0);
    // Employee 1 now has 1 shift -> score goes from -2 to -1, delta = +1
    assert_eq!(delta, SoftScore::of(1));

    // Insert shift at index 0 (employee 1)
    let delta = constraint.on_insert(&solution, 0, 0);
    // Employee 1 now has 2 shifts -> score goes from -1 to -2, delta = -1
    assert_eq!(delta, SoftScore::of(-1));
}

#[test]
fn test_grouped_constraint_reward() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Collaboration"),
        ImpactType::Reward,
        vec(|s: &GroupedSolution| &s.shifts),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
        ],
    };

    // 2 shifts in one group -> reward of +2
    assert_eq!(constraint.evaluate(&solution), SoftScore::of(2));
}

#[test]
fn test_grouped_constraint_weight_can_use_key() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Key weighted workload"),
        ImpactType::Penalty,
        vec(|s: &GroupedSolution| &s.shifts),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        count(),
        |employee_id: &usize, count: &usize| SoftScore::of((*employee_id as i64) * (*count as i64)),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-5));
}

#[test]
fn test_grouped_constraint_collect_vec_accepts_owned_labels() {
    let constraint = GroupedUniConstraint::new(
        ConstraintRef::new("", "Grouped labels"),
        ImpactType::Penalty,
        vec(|s: &GroupedSolution| &s.shifts),
        TrueFilter,
        |shift: &GroupedShift| shift.employee_id,
        collect_vec(|shift: &GroupedShift| format!("employee-{}", shift.employee_id)),
        |_employee_id: &usize, labels: &CollectedVec<String>| SoftScore::of(labels.len() as i64),
        false,
    );

    let solution = GroupedSolution {
        shifts: vec![
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 1 },
            GroupedShift { employee_id: 2 },
        ],
    };

    assert_eq!(constraint.evaluate(&solution), SoftScore::of(-3));
}
