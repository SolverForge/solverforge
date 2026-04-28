use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::IncrementalCrossBiConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;
use solverforge_core::score::{Score, SoftScore};
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Employee {
    id: usize,
    unavailable_days: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Shift {
    employee_id: Option<usize>,
    day: u32,
}

#[derive(Clone)]
struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
}

fn create_unavailable_employee_constraint() -> impl IncrementalConstraint<Schedule, SoftScore> {
    IncrementalCrossBiConstraint::new(
        ConstraintRef::new("", "Unavailable employee"),
        ImpactType::Penalty,
        source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ),
        source(
            (|schedule: &Schedule| schedule.employees.as_slice()) as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |_schedule: &Schedule, shift: &Shift, employee: &Employee| {
            shift.employee_id.is_some() && employee.unavailable_days.contains(&shift.day)
        },
        |_schedule: &Schedule, _shift_idx: usize, _employee_idx: usize| SoftScore::of(1),
        false,
    )
}

fn sample_schedule() -> Schedule {
    Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
                day: 5,
            },
            Shift {
                employee_id: Some(0),
                day: 6,
            },
        ],
        employees: vec![Employee {
            id: 0,
            unavailable_days: vec![5],
        }],
    }
}

#[test]
fn test_cross_bi_evaluate_works_without_initialize() {
    let constraint = create_unavailable_employee_constraint();
    let schedule = sample_schedule();

    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-1));
}

#[test]
fn test_cross_bi_match_count_works_without_initialize() {
    let constraint = create_unavailable_employee_constraint();
    let schedule = sample_schedule();

    assert_eq!(constraint.match_count(&schedule), 1);
}

#[test]
fn test_cross_bi_get_matches_works_without_initialize() {
    let constraint = create_unavailable_employee_constraint();
    let schedule = sample_schedule();

    let matches = constraint.get_matches(&schedule);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].constraint_ref.name, "Unavailable employee");
    assert_eq!(matches[0].score, SoftScore::of(-1));
    assert_eq!(matches[0].justification.entities.len(), 2);
    assert_eq!(
        matches[0].justification.entities[0]
            .as_entity::<Shift>()
            .unwrap(),
        &Shift {
            employee_id: Some(0),
            day: 5,
        }
    );
    assert_eq!(
        matches[0].justification.entities[1]
            .as_entity::<Employee>()
            .unwrap(),
        &Employee {
            id: 0,
            unavailable_days: vec![5],
        }
    );
}

#[test]
fn test_cross_bi_incremental_updates_still_work() {
    let mut constraint = create_unavailable_employee_constraint();
    let schedule = sample_schedule();

    let initial = constraint.initialize(&schedule);
    assert_eq!(initial, SoftScore::of(-1));

    let delta = constraint.on_retract(&schedule, 0, 0);
    assert_eq!(delta, SoftScore::of(1));

    let delta = constraint.on_insert(&schedule, 0, 0);
    assert_eq!(delta, SoftScore::of(-1));
}

#[test]
fn cross_bi_b_side_retract_and_insert_update_matches() {
    let mut constraint = create_unavailable_employee_constraint();
    let mut schedule = sample_schedule();

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&schedule, 0, 1);
    schedule.employees[0].unavailable_days = vec![6];
    total = total + constraint.on_insert(&schedule, 0, 1);

    assert_eq!(total, SoftScore::of(-1));
    assert_eq!(total, constraint.evaluate(&schedule));
}

#[test]
fn cross_bi_unrelated_descriptor_is_noop() {
    let mut constraint = create_unavailable_employee_constraint();
    let schedule = sample_schedule();

    let initial = constraint.initialize(&schedule);
    let delta = constraint.on_retract(&schedule, 0, 2);

    assert_eq!(initial, SoftScore::of(-1));
    assert_eq!(delta, SoftScore::zero());
}

#[test]
#[should_panic(expected = "cannot localize entity indexes")]
fn cross_bi_unknown_source_panics_on_localized_callback() {
    let mut constraint = IncrementalCrossBiConstraint::new(
        ConstraintRef::new("", "Unavailable employee"),
        ImpactType::Penalty,
        (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
        source(
            (|schedule: &Schedule| schedule.employees.as_slice()) as fn(&Schedule) -> &[Employee],
            ChangeSource::Descriptor(1),
        ),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |_schedule: &Schedule, shift: &Shift, employee: &Employee| {
            shift.employee_id.is_some() && employee.unavailable_days.contains(&shift.day)
        },
        |_schedule: &Schedule, _shift_idx: usize, _employee_idx: usize| SoftScore::of(1),
        false,
    );
    let schedule = sample_schedule();

    constraint.initialize(&schedule);
    constraint.on_insert(&schedule, 0, 0);
}

#[test]
fn keyed_join_accepts_unfiltered_source_aware_stream_target() {
    let mut constraint = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new().for_each(source(
                (|schedule: &Schedule| schedule.employees.as_slice())
                    as fn(&Schedule) -> &[Employee],
                ChangeSource::Static,
            )),
            equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .filter(|shift: &Shift, employee: &Employee| {
            shift.employee_id.is_some() && employee.unavailable_days.contains(&shift.day)
        })
        .penalize(SoftScore::of(1))
        .named("stream target join");

    let schedule = sample_schedule();
    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-1));

    total = total + constraint.on_retract(&schedule, 0, 0);
    assert_eq!(total, SoftScore::of(0));
    assert_eq!(constraint.on_insert(&schedule, 0, 1), SoftScore::zero());
}
