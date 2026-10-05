/* Chained equi-join operator: heterogeneous keys, row-aware left closures, cascade deltas.

Exercises the P2b `ChainedJoin` directly — no fluent adapters, no scoring
terminal. The first join relates assignments to shifts on `u32` shift ids;
the second relates the whole left row to employees on `String` employee
codes, with the left key inspecting the row. A second chained case uses a
combined key over both left bindings. Production results must agree with
the independent oracle on row multisets, and every supported change must
yield exact retract/insert deltas.
*/

use crate::stream::relational::{ChainedJoin, Concat, DeltaKind, EquiJoin, Leaf};

use super::fixtures::{
    rel_assignments, rel_employees, rel_shifts, sample, RelAssignment, RelEmployee, RelSchedule,
    RelShift,
};
use super::oracle::oracle_rows;
use super::updates::{assignment_key, assignment_shift_join, shift_key, AssignmentShiftJoin};

/* Row-aware left key: the employee code comes from the assignment binding.
Generic over the row lifetime so the operator's HRTB bound is satisfied by
a plain `fn` item and the operator type stays nameable. */
pub(super) fn row_employee_code(row: &Concat<Leaf<'_, RelAssignment>, RelShift>) -> String {
    row.left.entity.employee_code.clone()
}

pub(super) fn employee_key(employee: &RelEmployee) -> String {
    employee.code.clone()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn accept_all(
    _schedule: &RelSchedule,
    _assignment: &RelAssignment,
    _shift: &RelShift,
    _employee: &RelEmployee,
    _a_idx: usize,
    _b_idx: usize,
    _c_idx: usize,
) -> bool {
    true
}

pub(super) type NightStaffedChain = ChainedJoin<
    RelSchedule,
    RelAssignment,
    RelShift,
    RelEmployee,
    fn(&RelSchedule) -> &[RelAssignment],
    fn(&RelSchedule) -> &[RelShift],
    fn(&RelSchedule) -> &[RelEmployee],
    u32,
    fn(&RelAssignment) -> u32,
    fn(&RelShift) -> u32,
    String,
    fn(&Concat<Leaf<'_, RelAssignment>, RelShift>) -> String,
    fn(&RelEmployee) -> String,
    fn(&RelSchedule, &RelAssignment, &RelShift, usize, usize) -> bool,
    fn(&RelSchedule, &RelAssignment, &RelShift, &RelEmployee, usize, usize, usize) -> bool,
>;

pub(super) fn night_staffed_chain(schedule: &RelSchedule) -> NightStaffedChain {
    let first = assignment_shift_join(schedule);
    let mut chained = ChainedJoin::new(
        first,
        rel_employees as fn(&RelSchedule) -> &[RelEmployee],
        row_employee_code as fn(&Concat<Leaf<'_, RelAssignment>, RelShift>) -> String,
        employee_key as fn(&RelEmployee) -> String,
        accept_all
            as fn(
                &RelSchedule,
                &RelAssignment,
                &RelShift,
                &RelEmployee,
                usize,
                usize,
                usize,
            ) -> bool,
        1,
        0,
        2,
        "night staffed chain",
    );
    chained.refresh(schedule);
    chained
}

fn expected_triples() -> Vec<(usize, usize, usize)> {
    let mut rows: Vec<(usize, usize, usize)> = oracle_rows(&sample())
        .iter()
        .map(|row| (row.assignment_idx, row.shift_idx, row.employee_idx))
        .collect();
    rows.sort();
    rows
}

#[test]
fn chained_join_matches_oracle_triples_with_heterogeneous_keys() {
    let schedule = sample();
    // (a0, s0-night, e0) and (a2, s0-night, e0): u32 then String keys.
    let chained = night_staffed_chain(&schedule);
    assert_eq!(chained.output_rows(), expected_triples());
    assert_eq!(chained.output_rows(), vec![(0, 0, 0), (2, 0, 0)]);
}

#[allow(clippy::too_many_arguments)]
fn accept_pair(
    _schedule: &RelSchedule,
    _assignment: &RelAssignment,
    _shift: &RelShift,
    _a_idx: usize,
    _b_idx: usize,
) -> bool {
    true
}

#[test]
fn chained_join_left_key_combines_both_earlier_bindings() {
    // A left key over (assignment, shift) jointly: the tuple carries the
    // employee code AND the night flag. The first join runs WITHOUT the
    // night filter here, so the day pair (a1, s1-day) reaches the chain: a
    // code-only key would match (a1, s1, e1), but the combined key rejects
    // it on the night component. This proves the closure receives both
    // bindings rather than selecting one by type.
    fn combined_key(row: &Concat<Leaf<'_, RelAssignment>, RelShift>) -> (String, bool) {
        (
            row.left.entity.employee_code.clone(),
            row.right.entity.night,
        )
    }
    fn combined_right(employee: &RelEmployee) -> (String, bool) {
        (employee.code.clone(), true)
    }

    let schedule = sample();
    let mut first: AssignmentShiftJoin = EquiJoin::new(
        rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
        rel_shifts as fn(&RelSchedule) -> &[RelShift],
        assignment_key as fn(&RelAssignment) -> u32,
        shift_key as fn(&RelShift) -> u32,
        accept_pair as fn(&RelSchedule, &RelAssignment, &RelShift, usize, usize) -> bool,
        1,
        0,
        "unfiltered pairs",
    );
    first.refresh(&schedule);
    // All three assignment/shift pairs survive without the night filter.
    assert_eq!(first.output_rows(), vec![(0, 0), (1, 1), (2, 0)]);

    let mut chained = ChainedJoin::new(
        first,
        rel_employees as fn(&RelSchedule) -> &[RelEmployee],
        combined_key as fn(&Concat<Leaf<'_, RelAssignment>, RelShift>) -> (String, bool),
        combined_right as fn(&RelEmployee) -> (String, bool),
        accept_all
            as fn(
                &RelSchedule,
                &RelAssignment,
                &RelShift,
                &RelEmployee,
                usize,
                usize,
                usize,
            ) -> bool,
        1,
        0,
        2,
        "combined key chain",
    );
    chained.refresh(&schedule);
    // (a0, s0-night, e0) and (a2, s0-night, e0) match; (a1, s1-day, e1)
    // is rejected on the night component despite the code matching.
    assert_eq!(chained.output_rows(), vec![(0, 0, 0), (2, 0, 0)]);
}

#[test]
fn chained_join_cascades_left_retracts_exactly_once() {
    let schedule = sample();
    let mut chained = night_staffed_chain(&schedule);
    assert_eq!(chained.output_rows().len(), 2);

    // Retract assignment a0 (descriptor 1): the first join drops (a0, s0),
    // and the chain drops (a0, s0, e0) exactly once. (a2, s0, e0) survives.
    let deltas = chained.on_retract(1, 0);
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].kind, DeltaKind::Retract);
    assert_eq!(
        (deltas[0].a_idx, deltas[0].b_idx, deltas[0].c_idx),
        (0, 0, 0)
    );
    assert_eq!(chained.output_rows(), vec![(2, 0, 0)]);

    // Insert it back: exactly one chained insert restores the triple.
    let deltas = chained.on_insert(&schedule, 1, 0);
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].kind, DeltaKind::Insert);
    assert_eq!(
        (deltas[0].a_idx, deltas[0].b_idx, deltas[0].c_idx),
        (0, 0, 0)
    );
    assert_eq!(chained.output_rows(), vec![(0, 0, 0), (2, 0, 0)]);
}

#[test]
fn chained_join_right_retracts_hit_every_matching_output_once() {
    let schedule = sample();
    let mut chained = night_staffed_chain(&schedule);

    // Retract employee e0 (descriptor 2): both triples retract exactly once.
    let deltas = chained.on_retract(2, 0);
    assert_eq!(deltas.len(), 2);
    assert!(deltas.iter().all(|d| d.kind == DeltaKind::Retract));
    assert!(chained.output_rows().is_empty());

    // Unknown descriptors are exact no-ops on both paths.
    assert!(chained.on_retract(9, 0).is_empty());
    assert!(chained.on_insert(&schedule, 9, 0).is_empty());
}
