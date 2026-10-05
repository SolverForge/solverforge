/* Facade publication of the independent-key chained join.

Exercises the shipped P3 fluent surface through `solverforge::` paths:
heterogeneous successive keys (u32 shift ids, then String employee codes)
with a row-aware second key, a hard weight, and director-style
initialize/retract/insert deltas. The trybuild twin only proves
compilation under the attribute; this test proves scores and mutations.
*/

use solverforge::prelude::*;
use solverforge::stream::relational::operator::Pair;
use solverforge::stream::relational::Leaf;
use solverforge::stream::{source, ChangeSource};
use solverforge::IncrementalConstraint;

#[derive(Clone, Debug)]
struct RelShift {
    id: u32,
    night: bool,
}

#[derive(Clone, Debug)]
struct RelAssignment {
    shift_id: u32,
    employee_code: String,
}

#[derive(Clone, Debug)]
struct RelEmployee {
    code: String,
}

#[derive(Clone)]
struct RelSchedule {
    shifts: Vec<RelShift>,
    assignments: Vec<RelAssignment>,
    employees: Vec<RelEmployee>,
}

fn rel_shifts(schedule: &RelSchedule) -> &[RelShift] {
    schedule.shifts.as_slice()
}

fn rel_assignments(schedule: &RelSchedule) -> &[RelAssignment] {
    schedule.assignments.as_slice()
}

fn rel_employees(schedule: &RelSchedule) -> &[RelEmployee] {
    schedule.employees.as_slice()
}

fn assignment_shift_id(assignment: &RelAssignment) -> u32 {
    assignment.shift_id
}

fn shift_id(shift: &RelShift) -> u32 {
    shift.id
}

fn row_employee_code(row: &Pair<Leaf<'_, RelAssignment>, Leaf<'_, RelShift>>) -> String {
    row.left.entity.employee_code.clone()
}

fn employee_leaf_code(employee: &Leaf<'_, RelEmployee>) -> String {
    employee.entity.code.clone()
}

fn night_filter(assignment: &RelAssignment, shift: &RelShift, _employee: &RelEmployee) -> bool {
    let _ = assignment;
    shift.night
}

fn unit_weight(
    _assignment: &RelAssignment,
    _shift: &RelShift,
    _employee: &RelEmployee,
) -> HardSoftScore {
    HardSoftScore::of(0, 1)
}

fn staffed_constraint() -> impl IncrementalConstraint<RelSchedule, HardSoftScore> {
    ConstraintFactory::<RelSchedule, HardSoftScore>::new()
        .for_each(source(
            rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
            ChangeSource::Descriptor(1),
        ))
        .join((
            source(
                rel_shifts as fn(&RelSchedule) -> &[RelShift],
                ChangeSource::Descriptor(0),
            ),
            joiner::equal_bi(
                assignment_shift_id as fn(&RelAssignment) -> u32,
                shift_id as fn(&RelShift) -> u32,
            ),
        ))
        .join((
            source(
                rel_employees as fn(&RelSchedule) -> &[RelEmployee],
                ChangeSource::Descriptor(2),
            ),
            joiner::equal_on(
                row_employee_code
                    as fn(&Pair<Leaf<'_, RelAssignment>, Leaf<'_, RelShift>>) -> String,
                employee_leaf_code as fn(&Leaf<'_, RelEmployee>) -> String,
            ),
        ))
        .filter(night_filter as fn(&RelAssignment, &RelShift, &RelEmployee) -> bool)
        .penalize(unit_weight as fn(&RelAssignment, &RelShift, &RelEmployee) -> HardSoftScore)
        .named("night shift staffed")
}

fn sample() -> RelSchedule {
    RelSchedule {
        shifts: vec![
            RelShift {
                id: 10,
                night: true,
            },
            RelShift {
                id: 11,
                night: false,
            },
        ],
        assignments: vec![
            RelAssignment {
                shift_id: 10,
                employee_code: "e0".to_string(),
            },
            RelAssignment {
                shift_id: 11,
                employee_code: "e1".to_string(),
            },
            RelAssignment {
                shift_id: 10,
                employee_code: "e0".to_string(),
            },
        ],
        employees: vec![
            RelEmployee {
                code: "e0".to_string(),
            },
            RelEmployee {
                code: "e1".to_string(),
            },
        ],
    }
}

/* The README chain written with inline closures rather than fn items, so the
documented form is exercised, not just the fn-pointer form above. */
fn closure_chain_constraint() -> impl IncrementalConstraint<RelSchedule, SoftScore> {
    ConstraintFactory::<RelSchedule, SoftScore>::new()
        .for_each(source(
            rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
            ChangeSource::Descriptor(1),
        ))
        .join((
            source(
                rel_shifts as fn(&RelSchedule) -> &[RelShift],
                ChangeSource::Descriptor(0),
            ),
            joiner::equal_bi(|a: &RelAssignment| a.shift_id, |s: &RelShift| s.id),
        ))
        .join((
            source(
                rel_employees as fn(&RelSchedule) -> &[RelEmployee],
                ChangeSource::Descriptor(2),
            ),
            joiner::equal_on(
                |row: &Pair<Leaf<'_, RelAssignment>, Leaf<'_, RelShift>>| {
                    row.left.entity.employee_code.clone()
                },
                |e: &Leaf<'_, RelEmployee>| e.entity.code.clone(),
            ),
        ))
        .filter(|_a: &RelAssignment, s: &RelShift, _e: &RelEmployee| s.night)
        .penalize(SoftScore::of(1))
        .named("night shift staffed (closures)")
}

#[test]
fn readme_closure_chain_compiles_and_scores() {
    let schedule = sample();
    let constraint = closure_chain_constraint();
    assert_eq!(constraint.match_count(&schedule), 2);
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
}

#[test]
fn relational_chain_publishes_hard_scores_through_facade() {
    let schedule = sample();
    let mut constraint = staffed_constraint();
    // Two night triples on the hard... soft leg: (a0, s10, e0), (a2, s10, e0).
    assert_eq!(constraint.initialize(&schedule), HardSoftScore::of(0, -2));
    assert_eq!(constraint.evaluate(&schedule), HardSoftScore::of(0, -2));
    assert_eq!(constraint.match_count(&schedule), 2);

    // Retract assignment a0: one triple retracts.
    let delta = constraint.on_retract(&schedule, 0, 1);
    assert_eq!(delta, HardSoftScore::of(0, 1));

    // Re-insert: the triple returns.
    let delta = constraint.on_insert(&schedule, 0, 1);
    assert_eq!(delta, HardSoftScore::of(0, -1));
    assert_eq!(constraint.evaluate(&schedule), HardSoftScore::of(0, -2));
}
