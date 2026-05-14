// Tests for ComplementedGroupConstraint.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::complemented::ComplementedGroupConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::count;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Hash, PartialEq, Eq)]
struct Employee {
    id: usize,
}

#[derive(Clone)]
struct Shift {
    employee_id: Option<usize>,
}

#[derive(Clone)]
struct Schedule {
    employees: Vec<Employee>,
    shifts: Vec<Shift>,
}

fn shifts(s: &Schedule) -> &[Shift] {
    s.shifts.as_slice()
}

fn employees(s: &Schedule) -> &[Employee] {
    s.employees.as_slice()
}

#[test]
fn test_complemented_evaluate() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
    // Total: -2
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
}

#[test]
fn test_complemented_skips_none_keys() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift { employee_id: None }, // Unassigned - should be skipped
            Shift { employee_id: None }, // Unassigned - should be skipped
        ],
    };

    /* Only 2 assigned shifts count, both to employee 0
    Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
    Total: -2 (unassigned shifts don't count)
    */
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
}

#[test]
fn test_complemented_incremental() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Initialize
    let total = constraint.initialize(&schedule);
    /* Employee 0: 2 shifts -> -2
    Employee 1: 1 shift -> -1
    Employee 2: 0 shifts -> 0
    Total: -3
    */
    assert_eq!(total, SoftScore::of(-3));

    // Retract shift at index 0 (employee 0)
    let delta = constraint.on_retract(&schedule, 0, 0);
    // Employee 0 now has 1 shift -> score goes from -2 to -1, delta = +1
    assert_eq!(delta, SoftScore::of(1));

    // Insert shift at index 0 (employee 0)
    let delta = constraint.on_insert(&schedule, 0, 0);
    // Employee 0 now has 2 shifts -> score goes from -1 to -2, delta = -1
    assert_eq!(delta, SoftScore::of(-1));
}

#[test]
fn test_complemented_incremental_with_none_keys() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift { employee_id: None }, // Unassigned
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    // Initialize - only assigned shifts count
    let total = constraint.initialize(&schedule);
    // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0
    // Total: -2
    assert_eq!(total, SoftScore::of(-2));

    // Retract unassigned shift at index 1 - should be no-op
    let delta = constraint.on_retract(&schedule, 1, 0);
    assert_eq!(delta, SoftScore::of(0));

    // Insert unassigned shift at index 1 - should be no-op
    let delta = constraint.on_insert(&schedule, 1, 0);
    assert_eq!(delta, SoftScore::of(0));
}

#[test]
fn test_complemented_with_default() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Workload balance"),
        ImpactType::Penalty,
        shifts,
        employees,
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of((*count as i64).pow(2)),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    };

    /* Employee 0: 3 shifts -> 9
    Employee 1: 0 shifts -> 0
    Employee 2: 0 shifts -> 0
    Total penalty: -9
    */
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-9));
}

#[test]
fn test_complemented_incremental_matches_evaluate() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of((*count as i64).pow(2)),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
    };

    // Verify initialize matches evaluate
    let init_total = constraint.initialize(&schedule);
    let eval_total = constraint.evaluate(&schedule);
    assert_eq!(init_total, eval_total);

    // Employee 0: 2 shifts -> 4, Employee 1: 1 shift -> 1
    // Total: -5
    assert_eq!(init_total, SoftScore::of(-5));

    // Simulate retract + insert cycle and verify total remains consistent
    let mut running_total = init_total;

    // Retract shift 2 (employee 1)
    running_total = running_total + constraint.on_retract(&schedule, 2, 0);
    // Now: Employee 0: 2->4, Employee 1: 0->0, Total: -4
    assert_eq!(running_total, SoftScore::of(-4));

    // Insert shift 2 back (employee 1)
    running_total = running_total + constraint.on_insert(&schedule, 2, 0);
    // Back to: Employee 0: 2->4, Employee 1: 1->1, Total: -5
    assert_eq!(running_total, SoftScore::of(-5));
}

#[test]
fn test_complemented_b_side_insert_and_retract() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let mut schedule = Schedule {
        employees: vec![Employee { id: 0 }],
        shifts: vec![Shift {
            employee_id: Some(0),
        }],
    };

    let mut running_total = constraint.initialize(&schedule);
    assert_eq!(running_total, SoftScore::of(-1));

    running_total = running_total + constraint.on_retract(&schedule, 0, 1);
    schedule.employees[0].id = 2;
    running_total = running_total + constraint.on_insert(&schedule, 0, 1);

    assert_eq!(running_total, SoftScore::of(0));
    assert_eq!(running_total, constraint.evaluate(&schedule));
}

#[test]
fn test_complemented_missing_group_weight_can_use_complement_key() {
    let constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Missing key weighted shift count"),
        ImpactType::Penalty,
        shifts,
        employees,
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 0usize,
        |employee_id: &usize, count: &usize| SoftScore::of((*employee_id as i64) + (*count as i64)),
        false,
    );

    let schedule = Schedule {
        employees: vec![Employee { id: 1 }, Employee { id: 3 }],
        shifts: vec![Shift {
            employee_id: Some(1),
        }],
    };

    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-5));
}

#[test]
fn test_complemented_duplicate_complement_keys_match_incremental() {
    let mut constraint = ComplementedGroupConstraint::new(
        ConstraintRef::new("", "Duplicate complement key shift count"),
        ImpactType::Penalty,
        source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| emp.id,
        count(),
        |_emp: &Employee| 5usize,
        |_employee_id: &usize, count: &usize| SoftScore::of(*count as i64),
        false,
    );

    let mut schedule = Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![Shift {
            employee_id: Some(0),
        }],
    };

    let mut running_total = constraint.initialize(&schedule);
    assert_eq!(running_total, SoftScore::of(-7));
    assert_eq!(running_total, constraint.evaluate(&schedule));

    running_total = running_total + constraint.on_retract(&schedule, 0, 0);
    schedule.shifts[0].employee_id = None;
    running_total = running_total + constraint.on_insert(&schedule, 0, 0);

    assert_eq!(running_total, SoftScore::of(-15));
    assert_eq!(running_total, constraint.evaluate(&schedule));

    running_total = running_total + constraint.on_retract(&schedule, 0, 0);
    schedule.shifts[0].employee_id = Some(0);
    running_total = running_total + constraint.on_insert(&schedule, 0, 0);

    assert_eq!(running_total, SoftScore::of(-7));
    assert_eq!(running_total, constraint.evaluate(&schedule));
}
