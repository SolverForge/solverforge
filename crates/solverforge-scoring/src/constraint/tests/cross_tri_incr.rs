use std::sync::Arc;

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType, Score};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::cross_tri_incremental::Tri as CrossTriConstraint;
use crate::stream::collection_extract::{source, ChangeSource};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Shift {
    id: usize,
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Employee {
    id: usize,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct DayOff {
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone)]
struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
    days_off: Vec<DayOff>,
}

fn shifts_src(schedule: &Schedule) -> &[Shift] {
    schedule.shifts.as_slice()
}

fn employees_src(schedule: &Schedule) -> &[Employee] {
    schedule.employees.as_slice()
}

fn days_off_src(schedule: &Schedule) -> &[DayOff] {
    schedule.days_off.as_slice()
}

fn tri_constraint() -> impl IncrementalConstraint<Schedule, SoftScore> {
    CrossTriConstraint::new(
        ConstraintRef::new("", "Shift on day off"),
        ImpactType::Penalty,
        source(
            shifts_src as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees_src as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        source(
            days_off_src as fn(&Schedule) -> &[DayOff],
            ChangeSource::Descriptor(2),
        ),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |day_off: &DayOff| day_off.employee_id,
        |_s: &Schedule,
         shift: &Shift,
         _employee: &Employee,
         day_off: &DayOff,
         _a: usize,
         _b: usize,
         _c: usize| { shift.day == day_off.day },
        |_s: &Schedule, _a: usize, _b: usize, _c: usize| SoftScore::of(1),
        false,
    )
}

fn sample_schedule() -> Schedule {
    Schedule {
        shifts: vec![
            Shift {
                id: 0,
                employee_id: Some(0),
                day: 1,
            },
            Shift {
                id: 1,
                employee_id: Some(1),
                day: 2,
            },
            Shift {
                id: 2,
                employee_id: Some(0),
                day: 3,
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        days_off: vec![
            DayOff {
                employee_id: Some(0),
                day: 1,
            },
            DayOff {
                employee_id: Some(1),
                day: 5,
            },
        ],
    }
}

#[test]
fn cross_tri_evaluate_matches_expected_rows() {
    let constraint = tri_constraint();
    let schedule = sample_schedule();
    // (shift 0, emp 0, dayoff 0): key Some(0)==Some(0)==Some(0), day 1 == 1 → match.
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(constraint.match_count(&schedule), 1);
}

#[test]
fn cross_tri_initialize_then_incremental_matches_evaluate() {
    let schedule = sample_schedule();
    let mut constraint = tri_constraint();
    assert_eq!(constraint.initialize(&schedule), SoftScore::of(-1));

    // B-side change: adding employee 2 with a day-off row creates no new match.
    let extended = Schedule {
        shifts: schedule.shifts.clone(),
        employees: vec![Employee { id: 0 }, Employee { id: 1 }, Employee { id: 2 }],
        days_off: schedule.days_off.clone(),
    };
    let delta = constraint.on_insert(&extended, 2, 1);
    assert_eq!(delta, SoftScore::zero());
    assert_eq!(constraint.evaluate(&extended), SoftScore::of(-1));

    // C-side change: day off for employee 0 on day 3 matches shift 2.
    let with_day_off = Schedule {
        shifts: schedule.shifts.clone(),
        employees: schedule.employees.clone(),
        days_off: vec![
            DayOff {
                employee_id: Some(0),
                day: 1,
            },
            DayOff {
                employee_id: Some(1),
                day: 5,
            },
            DayOff {
                employee_id: Some(0),
                day: 3,
            },
        ],
    };
    let delta = constraint.on_insert(&with_day_off, 2, 2);
    assert_eq!(delta, SoftScore::of(-1));
    assert_eq!(constraint.evaluate(&with_day_off), SoftScore::of(-2));
    assert_eq!(constraint.match_count(&with_day_off), 2);

    // Retract it again; score returns to the initialized value. Evaluate
    // recomputes from the solution, so the retracted day-off row must be
    // physically absent there.
    let delta = constraint.on_retract(&with_day_off, 2, 2);
    assert_eq!(delta, SoftScore::of(1));
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
    assert_eq!(constraint.match_count(&schedule), 1);
}

#[test]
fn cross_tri_unrelated_descriptor_is_noop() {
    let schedule = sample_schedule();
    let mut constraint = tri_constraint();
    constraint.initialize(&schedule);
    assert_eq!(constraint.on_insert(&schedule, 0, 9), SoftScore::zero());
    assert_eq!(constraint.on_retract(&schedule, 0, 9), SoftScore::zero());
}

#[test]
#[should_panic(expected = "cannot localize entity indexes")]
fn cross_tri_unknown_source_panics_on_localized_callback() {
    // Shifts keep a raw fn extractor (ChangeSource::Unknown); a localized
    // callback for its reaction domain must trip the localization guard.
    let constraint = CrossTriConstraint::new(
        ConstraintRef::new("", "Shift on day off"),
        ImpactType::Penalty,
        (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
        source(
            employees_src as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        source(
            days_off_src as fn(&Schedule) -> &[DayOff],
            ChangeSource::Descriptor(2),
        ),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |day_off: &DayOff| day_off.employee_id,
        |_s: &Schedule,
         _shift: &Shift,
         _employee: &Employee,
         _day_off: &DayOff,
         _a: usize,
         _b: usize,
         _c: usize| true,
        |_s: &Schedule, _a: usize, _b: usize, _c: usize| SoftScore::of(1),
        false,
    );
    let schedule = sample_schedule();
    let mut constraint = constraint;
    constraint.initialize(&schedule);
    constraint.on_insert(&schedule, 0, 0);
}

#[test]
fn cross_tri_filter_receives_semantic_source_indexes() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = seen.clone();
    let constraint = CrossTriConstraint::new(
        ConstraintRef::new("", "Indexed tri rows"),
        ImpactType::Penalty,
        source(
            shifts_src as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            employees_src as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        source(
            days_off_src as fn(&Schedule) -> &[DayOff],
            ChangeSource::Descriptor(2),
        ),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |day_off: &DayOff| day_off.employee_id,
        move |_s: &Schedule,
              _shift: &Shift,
              _employee: &Employee,
              _day_off: &DayOff,
              a_idx: usize,
              b_idx: usize,
              c_idx: usize| {
            sink.lock().unwrap().push((a_idx, b_idx, c_idx));
            true
        },
        |_s: &Schedule, _a: usize, _b: usize, _c: usize| SoftScore::of(1),
        false,
    );
    let schedule = sample_schedule();
    // Keys gate the enumeration: employee-0 shifts pair with the employee-0
    // day-off row, employee-1 shift with the employee-1 row.
    let _ = constraint.evaluate(&schedule);
    let rows = seen.lock().unwrap().clone();
    assert_eq!(rows, vec![(0, 0, 0), (1, 1, 1), (2, 0, 0)]);
}
