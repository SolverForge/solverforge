/* Facade publication of the independent-key chained join.

Exercises the shipped P3 fluent surface through `solverforge::` paths:
heterogeneous successive keys (u32 shift ids, then String employee codes)
with a row-aware second key, a hard weight, and director-style
initialize/retract/insert deltas. The trybuild twin only proves
compilation under the attribute; this test proves scores and mutations.
*/

use solverforge::prelude::*;
use solverforge::stream::relational::{Concat, Leaf};
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

fn row_employee_code(row: &Concat<Leaf<'_, RelAssignment>, RelShift>) -> String {
    row.left.entity.employee_code.clone()
}

fn employee_code(employee: &RelEmployee) -> String {
    employee.code.clone()
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
        .join_on((
            source(
                rel_employees as fn(&RelSchedule) -> &[RelEmployee],
                ChangeSource::Descriptor(2),
            ),
            joiner::equal_on(
                row_employee_code as fn(&Concat<Leaf<'_, RelAssignment>, RelShift>) -> String,
                employee_code as fn(&RelEmployee) -> String,
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
