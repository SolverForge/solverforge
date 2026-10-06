// Complemented grouped behavior on the fluent surface the runtime uses.
//
// These exercise `group_by(...).complement(...).named(...)`, which finalizes
// into a `ComplementNode` + `GroupNode` operator tree through the shared
// `OperatorTerminal`, rather than the retired `complemented::Grouped`.
//
// Rows without a key group under `usize::MAX` and find no complement target,
// so they contribute nothing — the same workaround the canonical examples use.

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::collector::count;
use crate::stream::ConstraintFactory;
use solverforge_core::score::SoftScore;

const NO_KEY: usize = usize::MAX;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct Employee {
    id: usize,
}

#[derive(Clone, Debug)]
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

// Count shifts per employee, defaulting employees absent from the groups to 0.
fn count_constraint() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .group_by(|s: &Shift| s.employee_id.unwrap_or(NO_KEY), count())
        .complement(
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            |e: &Employee| e.id,
            |_e| 0usize,
        )
        .penalize(|_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Shift count")
}

fn schedule() -> Schedule {
    Schedule {
        employees: vec![Employee { id: 0 }, Employee { id: 1 }],
        shifts: vec![
            Shift {
                employee_id: Some(0),
            },
            Shift {
                employee_id: Some(0),
            },
        ],
    }
}

#[test]
fn complement_defaults_absent_keys() {
    // Employee 0: 2 shifts -> -2, Employee 1: 0 shifts -> 0. Total -2.
    assert_eq!(count_constraint().evaluate(&schedule()), SoftScore::of(-2));
}

#[test]
fn unassigned_rows_do_not_count() {
    let mut s = schedule();
    s.shifts.push(Shift { employee_id: None });
    s.shifts.push(Shift { employee_id: None });
    // Unassigned rows group under NO_KEY, which no employee matches.
    assert_eq!(count_constraint().evaluate(&s), SoftScore::of(-2));
}

#[test]
fn incremental_a_side_matches_evaluate() {
    let mut c = count_constraint();
    let mut s = schedule();
    let mut running = c.initialize(&s);
    assert_eq!(running, c.evaluate(&s)); // -2

    // Move shift 1 from employee 0 to employee 1: both count 1, total -2.
    running = running + c.on_retract(&s, 1, 0);
    s.shifts[1].employee_id = Some(1);
    running = running + c.on_insert(&s, 1, 0);
    assert_eq!(running, c.evaluate(&s));
}

#[test]
fn incremental_unassigned_transition_matches_evaluate() {
    let mut c = count_constraint();
    let mut s = schedule();
    let mut running = c.initialize(&s);
    assert_eq!(running, c.evaluate(&s));

    // Reassign shift 0 to unassigned: its penalty is released.
    running = running + c.on_retract(&s, 0, 0);
    s.shifts[0].employee_id = None;
    running = running + c.on_insert(&s, 0, 0);
    assert_eq!(running, c.evaluate(&s)); // employee 0 count 1 -> -1
}

#[test]
fn incremental_b_side_insert_and_retract_matches_evaluate() {
    let mut c = count_constraint();
    let mut s = schedule();
    let mut running = c.initialize(&s);

    // A third employee with no shifts contributes its default (0).
    s.employees.push(Employee { id: 2 });
    running = running + c.on_insert(&s, 2, 1);
    assert_eq!(running, c.evaluate(&s));

    s.employees.pop();
    running = running + c.on_retract(&s, 2, 1);
    assert_eq!(running, c.evaluate(&s));
}

#[test]
fn custom_default_is_honored() {
    let c = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .group_by(|s: &Shift| s.employee_id.unwrap_or(NO_KEY), count())
        .complement(
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            |e: &Employee| e.id,
            |_e| 5usize, // absent employees contribute 5
        )
        .penalize(|_id: &usize, count: &usize| SoftScore::of(*count as i64))
        .named("Shift count with default");
    // Employee 0: 2 -> -2, Employee 1: default 5 -> -5. Total -7.
    assert_eq!(c.evaluate(&schedule()), SoftScore::of(-7));
}

#[test]
fn weight_can_use_the_complement_key() {
    let c = ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(source(
            shifts as fn(&Schedule) -> &[Shift],
            ChangeSource::Descriptor(0),
        ))
        .group_by(|s: &Shift| s.employee_id.unwrap_or(NO_KEY), count())
        .complement(
            source(
                employees as fn(&Schedule) -> &[Employee],
                ChangeSource::Descriptor(1),
            ),
            |e: &Employee| e.id,
            |_e| 0usize,
        )
        .penalize(|id: &usize, count: &usize| SoftScore::of((*id as i64 + 1) * *count as i64))
        .named("Weighted shift count");
    // Employee 0: (0+1)*2 = 2, Employee 1: (1+1)*0 = 0. Total -2.
    assert_eq!(c.evaluate(&schedule()), SoftScore::of(-2));
}

#[test]
fn duplicate_complement_keys_emit_one_row_per_target() {
    let mut c = count_constraint();
    let mut s = schedule();
    // Two employees share id 1: both rows carry the same (default) result.
    s.employees.push(Employee { id: 1 });
    let running = c.initialize(&s);
    assert_eq!(running, c.evaluate(&s));
    assert_eq!(c.match_count(&s), 3);
}
