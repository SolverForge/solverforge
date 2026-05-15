use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::IncrementalCrossBiConstraint;
use crate::stream::collection_extract::{source, ChangeSource, CollectionExtract};
use crate::stream::collector::sum;
use crate::stream::filter::FnBiFilter;
use crate::stream::joiner::equal_bi;
use crate::stream::{ConstraintFactory, CrossBiConstraintStream};
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

#[derive(Clone)]
struct CountingShiftExtract {
    calls: Arc<AtomicUsize>,
}

impl CollectionExtract<Schedule> for CountingShiftExtract {
    type Item = Shift;

    fn extract<'s>(&self, schedule: &'s Schedule) -> &'s [Self::Item] {
        self.calls.fetch_add(1, Ordering::Relaxed);
        schedule.shifts.as_slice()
    }

    fn change_source(&self) -> ChangeSource {
        ChangeSource::Descriptor(0)
    }
}

#[derive(Clone)]
struct CountingEmployeeExtract {
    calls: Arc<AtomicUsize>,
}

impl CollectionExtract<Schedule> for CountingEmployeeExtract {
    type Item = Employee;

    fn extract<'s>(&self, schedule: &'s Schedule) -> &'s [Self::Item] {
        self.calls.fetch_add(1, Ordering::Relaxed);
        schedule.employees.as_slice()
    }

    fn change_source(&self) -> ChangeSource {
        ChangeSource::Descriptor(1)
    }
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
        |_schedule: &Schedule,
         shift: &Shift,
         employee: &Employee,
         _shift_idx: usize,
         _employee_idx: usize| {
            shift.employee_id.is_some() && employee.unavailable_days.contains(&shift.day)
        },
        |_schedule: &Schedule, _shift_idx: usize, _employee_idx: usize| SoftScore::of(1),
        false,
    )
}

fn create_grouped_shift_count_constraint() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            source(
                (|schedule: &Schedule| schedule.employees.as_slice())
                    as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .group_by(
            |_shift: &Shift, employee: &Employee| employee.id,
            sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
        )
        .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(count * count))
        .named("grouped assigned shift count")
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

fn two_employee_schedule() -> Schedule {
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
        employees: vec![
            Employee {
                id: 0,
                unavailable_days: Vec::new(),
            },
            Employee {
                id: 1,
                unavailable_days: Vec::new(),
            },
        ],
    }
}

#[test]
fn cross_bi_unrelated_insert_skips_extractors() {
    let shift_extract_calls = Arc::new(AtomicUsize::new(0));
    let employee_extract_calls = Arc::new(AtomicUsize::new(0));
    let mut constraint = IncrementalCrossBiConstraint::new(
        ConstraintRef::new("", "Unavailable employee"),
        ImpactType::Penalty,
        CountingShiftExtract {
            calls: Arc::clone(&shift_extract_calls),
        },
        CountingEmployeeExtract {
            calls: Arc::clone(&employee_extract_calls),
        },
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |_schedule: &Schedule,
         shift: &Shift,
         employee: &Employee,
         _shift_idx: usize,
         _employee_idx: usize| {
            shift.employee_id.is_some() && employee.unavailable_days.contains(&shift.day)
        },
        |_schedule: &Schedule, _shift_idx: usize, _employee_idx: usize| SoftScore::of(1),
        false,
    );
    let schedule = sample_schedule();

    assert_eq!(constraint.initialize(&schedule), SoftScore::of(-1));
    shift_extract_calls.store(0, Ordering::Relaxed);
    employee_extract_calls.store(0, Ordering::Relaxed);

    let delta = constraint.on_insert(&schedule, 0, 2);

    assert_eq!(delta, SoftScore::zero());
    assert_eq!(shift_extract_calls.load(Ordering::Relaxed), 0);
    assert_eq!(employee_extract_calls.load(Ordering::Relaxed), 0);
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
fn cross_bi_group_by_scores_joined_pairs_without_projection() {
    let constraint = create_grouped_shift_count_constraint();
    let schedule = two_employee_schedule();

    assert_eq!(constraint.match_count(&schedule), 1);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-4));
}

#[test]
fn cross_bi_group_by_incremental_updates_join_groups() {
    let mut constraint = create_grouped_shift_count_constraint();
    let mut schedule = two_employee_schedule();

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-4));

    total = total + constraint.on_retract(&schedule, 1, 0);
    schedule.shifts[1].employee_id = Some(1);
    total = total + constraint.on_insert(&schedule, 1, 0);

    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&schedule));
}

#[test]
fn cross_bi_direct_path_preserves_filter_source_indexes() {
    let constraint = CrossBiConstraintStream::new_with_filter(
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
        FnBiFilter::new(
            |_schedule: &Schedule,
             _shift: &Shift,
             _employee: &Employee,
             shift_idx: usize,
             employee_idx: usize| { shift_idx == 1 && employee_idx == 0 },
        ),
    )
    .penalize(|shift: &Shift, _employee: &Employee| SoftScore::of(shift.day as i64))
    .named("indexed cross path");

    let schedule = two_employee_schedule();

    assert_eq!(constraint.match_count(&schedule), 1);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-6));
}

#[test]
fn cross_bi_group_by_preserves_filter_source_indexes() {
    let constraint = CrossBiConstraintStream::new_with_filter(
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
        FnBiFilter::new(
            |_schedule: &Schedule,
             _shift: &Shift,
             _employee: &Employee,
             shift_idx: usize,
             employee_idx: usize| { shift_idx == 1 && employee_idx == 0 },
        ),
    )
    .group_by(
        |_shift: &Shift, employee: &Employee| employee.id,
        sum(|(shift, _employee): (&Shift, &Employee)| shift.day as i64),
    )
    .penalize(|_employee_id: &usize, total_day: &i64| SoftScore::of(*total_day))
    .named("indexed grouped cross path");

    let schedule = two_employee_schedule();

    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-6));
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
        |_schedule: &Schedule,
         shift: &Shift,
         employee: &Employee,
         _shift_idx: usize,
         _employee_idx: usize| {
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

#[test]
fn keyed_join_honors_filtered_source_aware_stream_target() {
    let mut constraint = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new()
                .for_each(source(
                    (|schedule: &Schedule| schedule.employees.as_slice())
                        as fn(&Schedule) -> &[Employee],
                    ChangeSource::Descriptor(1),
                ))
                .filter(|employee: &Employee| employee.id != 0),
            equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .filter(|shift: &Shift, employee: &Employee| {
            shift.employee_id.is_some() && employee.unavailable_days.contains(&shift.day)
        })
        .penalize(SoftScore::of(1))
        .named("filtered stream target join");

    let schedule = sample_schedule();
    assert_eq!(constraint.initialize(&schedule), SoftScore::zero());
}
