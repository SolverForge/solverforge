use crate::api::constraint_set::{ConstraintSet, IncrementalConstraint};
use crate::constraint::cross_complemented_grouped::{
    CrossComplementedGroupedNodeState, CrossComplementedGroupedTerminalScorer,
    SharedCrossComplementedGroupedConstraintSet,
};
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::sum;
use crate::stream::joiner::equal_bi;
use crate::stream::ConstraintFactory;
use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Employee {
    id: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Shift {
    employee_id: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Target {
    employee_id: usize,
}

#[derive(Clone)]
struct Schedule {
    shifts: Vec<Shift>,
    employees: Vec<Employee>,
    targets: Vec<Target>,
}

fn shift_source(
) -> impl crate::stream::collection_extract::CollectionExtract<Schedule, Item = Shift> {
    source(
        (|schedule: &Schedule| schedule.shifts.as_slice()) as fn(&Schedule) -> &[Shift],
        ChangeSource::Descriptor(0),
    )
}

fn employee_source(
) -> impl crate::stream::collection_extract::CollectionExtract<Schedule, Item = Employee> {
    source(
        (|schedule: &Schedule| schedule.employees.as_slice()) as fn(&Schedule) -> &[Employee],
        ChangeSource::Descriptor(1),
    )
}

fn target_source(
) -> impl crate::stream::collection_extract::CollectionExtract<Schedule, Item = Target> {
    source(
        (|schedule: &Schedule| schedule.targets.as_slice()) as fn(&Schedule) -> &[Target],
        ChangeSource::Descriptor(2),
    )
}

fn complemented_shift_count_constraint() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(shift_source())
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new().for_each(employee_source()),
            equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .group_by(
            |_shift: &Shift, employee: &Employee| employee.id,
            sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
        )
        .complement(
            target_source(),
            |target: &Target| target.employee_id,
            |_| 5i64,
        )
        .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(*count))
        .named("complemented cross grouped shift count")
}

fn filtered_stream_complemented_shift_count_constraint(
) -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(shift_source())
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new()
                .for_each(employee_source())
                .filter(|employee: &Employee| employee.id != 0),
            equal_bi(
                |shift: &Shift| shift.employee_id,
                |employee: &Employee| Some(employee.id),
            ),
        ))
        .group_by(
            |_shift: &Shift, employee: &Employee| employee.id,
            sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
        )
        .complement(
            ConstraintFactory::<Schedule, SoftScore>::new()
                .for_each(target_source())
                .filter(|target: &Target| target.employee_id != 2),
            |target: &Target| target.employee_id,
            |_| 5i64,
        )
        .penalize(|_employee_id: &usize, count: &i64| SoftScore::of(*count))
        .named("filtered complemented cross grouped shift count")
}

fn two_employee_schedule() -> Schedule {
    Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        targets: vec![Target { employee_id: 0 }, Target { employee_id: 1 }],
    }
}

fn three_target_schedule() -> Schedule {
    Schedule {
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(1),
            },
        ],
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        targets: vec![
            Target { employee_id: 0 },
            Target { employee_id: 1 },
            Target { employee_id: 2 },
        ],
    }
}

#[test]
fn cross_bi_group_by_complement_scores_missing_join_groups() {
    let constraint = complemented_shift_count_constraint();
    let schedule = two_employee_schedule();

    assert_eq!(constraint.match_count(&schedule), 2);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-7));
}

#[test]
fn cross_bi_group_by_complement_incrementally_updates_join_groups() {
    let mut constraint = complemented_shift_count_constraint();
    let mut schedule = two_employee_schedule();

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-7));

    total = total + constraint.on_retract(&schedule, 1, 0);
    schedule.shifts[1].employee_id = Some(1);
    total = total + constraint.on_insert(&schedule, 1, 0);

    assert_eq!(total, SoftScore::of(-2));
    assert_eq!(total, constraint.evaluate(&schedule));
}

#[test]
fn cross_bi_group_by_complement_incrementally_updates_join_right_source() {
    let mut constraint = complemented_shift_count_constraint();
    let mut schedule = two_employee_schedule();

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-7));

    total = total + constraint.on_retract(&schedule, 0, 1);
    schedule.employees[0].id = 2;
    total = total + constraint.on_insert(&schedule, 0, 1);

    assert_eq!(total, SoftScore::of(-10));
    assert_eq!(total, constraint.evaluate(&schedule));
}

#[test]
fn cross_bi_group_by_complement_incrementally_updates_complement_source() {
    let mut constraint = complemented_shift_count_constraint();
    let mut schedule = two_employee_schedule();

    let mut total = constraint.initialize(&schedule);
    schedule.targets.push(Target { employee_id: 2 });
    total = total + constraint.on_insert(&schedule, 2, 2);

    assert_eq!(total, SoftScore::of(-12));
    assert_eq!(total, constraint.evaluate(&schedule));
}

#[test]
fn cross_bi_group_by_complement_honors_filtered_join_and_complement_sources() {
    let mut constraint = filtered_stream_complemented_shift_count_constraint();
    let mut schedule = three_target_schedule();

    assert_eq!(constraint.match_count(&schedule), 2);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-6));

    let mut total = constraint.initialize(&schedule);
    assert_eq!(total, SoftScore::of(-6));

    total = total + constraint.on_retract(&schedule, 1, 1);
    schedule.employees[1].id = 0;
    total = total + constraint.on_insert(&schedule, 1, 1);

    assert_eq!(total, SoftScore::of(-10));
    assert_eq!(total, constraint.evaluate(&schedule));
}

#[test]
fn shared_cross_bi_group_by_complement_updates_one_node_for_multiple_terminals() {
    let state = CrossComplementedGroupedNodeState::new(
        shift_source(),
        employee_source(),
        target_source(),
        |shift: &Shift| shift.employee_id,
        |employee: &Employee| Some(employee.id),
        |_schedule: &Schedule, _shift: &Shift, _employee: &Employee, _a_idx, _b_idx| true,
        |_shift: &Shift, employee: &Employee| employee.id,
        |target: &Target| target.employee_id,
        sum(|(_shift, _employee): (&Shift, &Employee)| 1i64),
        |_target: &Target| 5i64,
    );
    let scorers = (
        CrossComplementedGroupedTerminalScorer::new(
            ConstraintRef::new("", "complemented shift count"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &i64| SoftScore::of(*count),
            false,
        ),
        CrossComplementedGroupedTerminalScorer::new(
            ConstraintRef::new("", "double complemented shift count"),
            ImpactType::Penalty,
            |_employee_id: &usize, count: &i64| SoftScore::of(*count * 2),
            false,
        ),
    );
    let mut constraints = SharedCrossComplementedGroupedConstraintSet::new(
        "shared complemented counts",
        state,
        scorers,
    );
    let mut schedule = two_employee_schedule();

    let mut total = constraints.initialize_all(&schedule);
    assert_eq!(total, SoftScore::of(-21));
    assert_eq!(constraints.state().update_count(), 0);

    total = total + constraints.on_retract_all(&schedule, 1, 0);
    schedule.shifts[1].employee_id = Some(1);
    total = total + constraints.on_insert_all(&schedule, 1, 0);

    assert_eq!(constraints.state().update_count(), 2);
    assert_eq!(total, SoftScore::of(-6));
    assert_eq!(total, constraints.evaluate_all(&schedule));
    assert_eq!(constraints.evaluate_each(&schedule).len(), 2);
}
