// Tests for FlattenedBiConstraint.

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::flattened_bi::FlattenedBiConstraint;
use crate::stream::collection_extract::{source, ChangeSource, SourceExtract};
use crate::stream::{joiner, ConstraintFactory};
use solverforge_core::score::SoftScore;
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
    SourceExtract<fn(&Schedule) -> &[Shift]>,
    SourceExtract<fn(&Schedule) -> &[Employee]>,
    impl Fn(&Shift) -> Option<usize>,
    impl Fn(&Employee) -> Option<usize>,
    impl Fn(&Employee) -> &[u32],
    impl Fn(&u32) -> u32,
    impl Fn(&Shift) -> u32,
    impl Fn(&Schedule, &Shift, &u32, usize, usize) -> bool,
    impl Fn(&Shift, &u32) -> SoftScore,
    SoftScore,
> {
    FlattenedBiConstraint::new(
        ConstraintRef::new("", "Unavailable employee"),
        ImpactType::Penalty,
        source(
            (|s: &Schedule| s.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            (|s: &Schedule| s.employees.as_slice()) as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| Some(emp.id),
        |emp: &Employee| emp.unavailable_days.as_slice(),
        |day: &u32| *day,
        |shift: &Shift| shift.day,
        |_s: &Schedule, shift: &Shift, day: &u32, _shift_idx: usize, _employee_idx: usize| {
            shift.employee_id.is_some() && shift.day == *day
        },
        |_shift: &Shift, _day: &u32| SoftScore::of(1),
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
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
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
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(0));
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
    assert_eq!(initial, SoftScore::of(-1));

    // Retract conflicting shift
    let delta = constraint.on_retract(&schedule, 0, 0);
    assert_eq!(delta, SoftScore::of(1)); // Removing penalty

    // Re-insert it
    let delta = constraint.on_insert(&schedule, 0, 0);
    assert_eq!(delta, SoftScore::of(-1)); // Adding penalty back
}

#[test]
fn flattened_filter_receives_a_index_and_owner_b_index() {
    let mut constraint = FlattenedBiConstraint::new(
        ConstraintRef::new("", "Indexed unavailable employee"),
        ImpactType::Penalty,
        source(
            (|s: &Schedule| s.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            (|s: &Schedule| s.employees.as_slice()) as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |emp: &Employee| Some(emp.id),
        |emp: &Employee| emp.unavailable_days.as_slice(),
        |day: &u32| *day,
        |shift: &Shift| shift.day,
        |_s: &Schedule, _shift: &Shift, _day: &u32, shift_idx: usize, employee_idx: usize| {
            shift_idx == 0 && employee_idx == 1
        },
        |_shift: &Shift, _day: &u32| SoftScore::of(1),
        false,
    );
    let schedule = Schedule {
        shifts: vec![Shift {
            employee_id: Some(0),
            day: 5,
        }],
        employees: vec![
            Employee {
                id: 0,
                unavailable_days: vec![4],
            },
            Employee {
                id: 0,
                unavailable_days: vec![5],
            },
        ],
    };

    assert_eq!(constraint.match_count(&schedule), 1);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(constraint.initialize(&schedule), SoftScore::of(-1));
}

#[test]
fn test_b_side_flattened_update_localizes_affected_a_scores() {
    let mut constraint = create_test_constraint();
    let mut schedule = Schedule {
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
            unavailable_days: vec![5],
        }],
    };

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&schedule, 0, 1);
    schedule.employees[0].unavailable_days = vec![10];
    total = total + constraint.on_insert(&schedule, 0, 1);

    assert_eq!(total, SoftScore::of(-1));
    assert_eq!(total, constraint.evaluate(&schedule));
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
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(0));
}

#[test]
fn flattened_keyed_join_honors_filtered_target_stream() {
    let mut constraint = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            (|s: &Schedule| s.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new()
                .for_each(source(
                    (|s: &Schedule| s.employees.as_slice()) as fn(&Schedule) -> &[Employee],
                    ChangeSource::Descriptor(1),
                ))
                .filter(|employee: &Employee| employee.id == 1),
            joiner::equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .flatten_last(
            |employee: &Employee| employee.unavailable_days.as_slice(),
            |day: &u32| *day,
            |shift: &Shift| shift.day,
        )
        .penalize(SoftScore::of(1))
        .named("filtered target flattened join");

    let mut schedule = Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                day: 5,
            },
            Shift {
                employee_id: Some(1),
                day: 5,
            },
        ],
        employees: vec![
            Employee {
                id: 0,
                unavailable_days: vec![5],
            },
            Employee {
                id: 1,
                unavailable_days: vec![5],
            },
        ],
    };

    assert_eq!(constraint.match_count(&schedule), 1);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&schedule, 1, 1);
    schedule.employees[1].id = 2;
    total = total + constraint.on_insert(&schedule, 1, 1);

    assert_eq!(total, SoftScore::of(0));
    assert_eq!(total, constraint.evaluate(&schedule));
}
