//! Tests for FlattenedBiConstraint.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::flattened_bi::FlattenedBiConstraint;
use solverforge_core::score::SimpleScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone)]
struct Employee {
    id: usize,
    unavailable_days: Vec<u32>,
}

#[derive(Clone)]
struct Shift {
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone)]
struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
}

fn create_test_constraint() -> FlattenedBiConstraint<
    Schedule,
    Shift,
    Employee,
    u32,
    Option<usize>,
    u32,
    impl Fn(&Schedule) -> &[Shift],
    impl Fn(&Schedule) -> &[Employee],
    impl Fn(&Shift) -> Option<usize>,
    impl Fn(&Employee) -> Option<usize>,
    impl Fn(&Employee) -> &[u32],
    impl Fn(&u32) -> u32,
    impl Fn(&Shift) -> u32,
    impl Fn(&Schedule, &Shift, &u32) -> bool,
    impl Fn(&Shift, &u32) -> SimpleScore,
    SimpleScore,
> {
    FlattenedBiConstraint::new(
        ConstraintRef::new("", "Unavailable employee"),
        ImpactType::Penalty,
        |s: &Schedule| s.shifts.as_slice(),
        |s: &Schedule| s.employees.as_slice(),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| Some(emp.id),
        |emp: &Employee| emp.unavailable_days.as_slice(),
        |day: &u32| *day,
        |shift: &Shift| shift.day,
        |_s: &Schedule, shift: &Shift, day: &u32| shift.employee_id.is_some() && shift.day == *day,
        |_shift: &Shift, _day: &u32| SimpleScore::of(1),
        false,
    )
}

#[test]
fn test_evaluate_single_match() {
    let constraint = create_test_constraint();
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                day: 5,
            },
            Shift {
                employee_id: Some(0),
                day: 10,
            },
        ],
        employees: vec![Employee {
            id: 0,
            unavailable_days: vec![5, 15],
        }],
    };

    // Day 5 shift conflicts with employee's unavailable day 5
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(-1));
}

#[test]
fn test_evaluate_no_match() {
    let constraint = create_test_constraint();
    let schedule = Schedule {
        shifts: vec![Shift {
            employee_id: Some(0),
            day: 10,
        }],
        employees: vec![Employee {
            id: 0,
            unavailable_days: vec![5, 15],
        }],
    };

    // Day 10 doesn't conflict
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(0));
}

#[test]
fn test_incremental() {
    let mut constraint = create_test_constraint();
    let schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                day: 5,
            }, // Conflicts
            Shift {
                employee_id: Some(0),
                day: 10,
            }, // No conflict
        ],
        employees: vec![Employee {
            id: 0,
            unavailable_days: vec![5, 15],
        }],
    };

    // Initialize
    let initial = constraint.initialize(&schedule);
    assert_eq!(initial, SimpleScore::of(-1));

    // Retract conflicting shift
    let delta = constraint.on_retract(&schedule, 0, 0);
    assert_eq!(delta, SimpleScore::of(1)); // Removing penalty

    // Re-insert it
    let delta = constraint.on_insert(&schedule, 0, 0);
    assert_eq!(delta, SimpleScore::of(-1)); // Adding penalty back
}

#[test]
fn test_unassigned_shift() {
    let constraint = create_test_constraint();
    let schedule = Schedule {
        shifts: vec![Shift {
            employee_id: None, // Unassigned
            day: 5,
        }],
        employees: vec![Employee {
            id: 0,
            unavailable_days: vec![5],
        }],
    };

    // Unassigned shift doesn't match
    assert_eq!(constraint.evaluate(&schedule), SimpleScore::of(0));
}
