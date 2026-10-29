use super::*;

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

#[test]
fn self_join_honors_filtered_source_for_both_entities() {
    let mut constraint = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .filter(|shift: &Shift| shift.day == 5)
        .join(crate::stream::joiner::equal(|shift: &Shift| {
            shift.employee_id
        }))
        .penalize(SoftScore::of(1))
        .named("filtered self join");

    let schedule = sample_schedule();
    assert_eq!(constraint.initialize(&schedule), SoftScore::zero());
}
