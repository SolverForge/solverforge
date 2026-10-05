/* P3 RED: independent-key chain through the public fluent stream API.

The first join relates assignments to shifts on `u32` shift ids; the
second relates the whole left row to employees on `String` employee
codes. Production must agree with the oracle on scores and match
counts. Fails until `Bi` exposes a row-aware second join with its own
key type (P3 adapters).
*/

use solverforge_core::score::SoftScore;

use crate::api::constraint_set::IncrementalConstraint;
use crate::stream::collection_extract::{source, ChangeSource};
use crate::stream::joiner::{equal_bi, equal_on};
use crate::stream::relational::{Concat, Leaf};
use crate::stream::ConstraintFactory;

use super::chained::row_employee_code;
use super::fixtures::{
    rel_assignments, rel_employees, rel_shifts, sample, RelAssignment, RelEmployee, RelSchedule,
    RelShift,
};
use super::oracle::{oracle_rows, oracle_score};
use super::updates::{assignment_key, shift_key};

fn fluent_night_staffed() -> impl IncrementalConstraint<RelSchedule, SoftScore> {
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
            equal_bi(
                assignment_key as fn(&RelAssignment) -> u32,
                shift_key as fn(&RelShift) -> u32,
            ),
        ))
        .join_on((
            source(
                rel_employees as fn(&RelSchedule) -> &[RelEmployee],
                ChangeSource::Descriptor(2),
            ),
            equal_on(
                row_employee_code as fn(&Concat<Leaf<'_, RelAssignment>, RelShift>) -> String,
                super::chained::employee_key as fn(&RelEmployee) -> String,
            ),
        ))
        .filter(
            |_assignment: &RelAssignment, shift: &RelShift, _employee: &RelEmployee| shift.night,
        )
        .penalize(
            |_assignment: &RelAssignment, _shift: &RelShift, _employee: &RelEmployee| {
                SoftScore::of(1)
            },
        )
        .named("night shift staffed")
}

#[test]
fn fluent_independent_key_chain_matches_oracle() {
    let schedule = sample();
    let constraint = fluent_night_staffed();
    assert_eq!(constraint.evaluate(&schedule), oracle_score(&schedule));
    assert_eq!(
        constraint.match_count(&schedule),
        oracle_rows(&schedule).len()
    );
    assert_eq!(oracle_rows(&schedule).len(), 2);
}

#[test]
fn fluent_chain_initialize_then_incremental_matches_evaluate() {
    let schedule = sample();
    let mut constraint = fluent_night_staffed();
    assert_eq!(constraint.initialize(&schedule), SoftScore::of(-2));

    // Retract assignment a0 (descriptor 1, index 0): one triple retracts.
    let delta = constraint.on_retract(&schedule, 0, 1);
    assert_eq!(delta, SoftScore::of(1));

    // Full recomputation on the trimmed solution agrees with retention.
    let trimmed = RelSchedule {
        shifts: schedule.shifts.clone(),
        assignments: schedule.assignments[1..].to_vec(),
        employees: schedule.employees.clone(),
        score: None,
    };
    assert_eq!(constraint.evaluate(&trimmed), SoftScore::of(-1));
    assert_eq!(constraint.match_count(&trimmed), 1);

    // Re-insert a0: -1 restores the total.
    let delta = constraint.on_insert(&schedule, 0, 1);
    assert_eq!(delta, SoftScore::of(-1));
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));

    // Explanations carry all three bindings with signed sums.
    let matches = constraint.get_matches(&schedule);
    assert_eq!(matches.len(), 2);
    let total: SoftScore = matches
        .iter()
        .fold(SoftScore::of(0), |acc, m| acc + m.score);
    assert_eq!(total, SoftScore::of(-2));
}
