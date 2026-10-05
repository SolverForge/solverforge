/* Standalone binary equi-join operator: independent keys, bag multiplicity, delta routing.

Exercises the P2 `stream::relational` source + equi-join operators
directly — no fluent adapters, no scoring terminal, no chaining. The
operator joins two collection sources on its own typed equality
relationship (u32 shift ids here; the String employee domain chains in
the next slice), retains output rows with stable identities, and routes
root descriptor notifications into exactly-once output deltas. Production
results must agree with the independent oracle on row multisets.
*/

use crate::stream::relational::{DeltaKind, EquiJoin};

use super::fixtures::{rel_assignments, rel_shifts, sample, RelAssignment, RelSchedule, RelShift};

/* Test-domain wiring: shifts own descriptor 0, assignments descriptor 1,
employees (chained next slice) descriptor 2. All key/filter functions are
`fn` items so the operator type stays nameable. */
pub(super) type AssignmentShiftJoin = EquiJoin<
    RelSchedule,
    RelAssignment,
    RelShift,
    fn(&RelSchedule) -> &[RelAssignment],
    fn(&RelSchedule) -> &[RelShift],
    u32,
    fn(&RelAssignment) -> u32,
    fn(&RelShift) -> u32,
    fn(&RelSchedule, &RelAssignment, &RelShift, usize, usize) -> bool,
>;

pub(super) fn assignment_key(assignment: &RelAssignment) -> u32 {
    assignment.shift_id
}

pub(super) fn shift_key(shift: &RelShift) -> u32 {
    shift.id
}

pub(super) fn night_only(
    _schedule: &RelSchedule,
    _assignment: &RelAssignment,
    shift: &RelShift,
    _a_idx: usize,
    _b_idx: usize,
) -> bool {
    shift.night
}

pub(super) fn assignment_shift_join(schedule: &RelSchedule) -> AssignmentShiftJoin {
    let mut join = EquiJoin::new(
        rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
        rel_shifts as fn(&RelSchedule) -> &[RelShift],
        assignment_key as fn(&RelAssignment) -> u32,
        shift_key as fn(&RelShift) -> u32,
        night_only as fn(&RelSchedule, &RelAssignment, &RelShift, usize, usize) -> bool,
        1,
        0,
        "night staffed",
    );
    join.refresh(schedule);
    join
}

fn assignment_shift_rows() -> Vec<(usize, usize)> {
    // Oracle rows restricted to (assignment, shift): dedupe employee side.
    use super::oracle::oracle_rows;
    use std::collections::BTreeSet;
    oracle_rows(&sample())
        .iter()
        .map(|row| (row.assignment_idx, row.shift_idx))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[test]
fn equi_join_matches_oracle_pair_rows() {
    let schedule = sample();
    // (assignment, shift) on assignment.shift_id == shift.id, residual
    // shift.night: (a0, s0), (a2, s0). Day shift s1 never joins.
    let join = assignment_shift_join(&schedule);
    let mut rows = join.output_rows();
    rows.sort();
    assert_eq!(rows, assignment_shift_rows());
    assert_eq!(rows, vec![(0, 0), (2, 0)]);
}

#[test]
fn equi_join_preserves_bag_multiplicity_for_duplicate_keys() {
    // Assignments a0 and a2 share shift_id 10 but are distinct rows; both
    // join to shift s0.
    let schedule = sample();
    let join = assignment_shift_join(&schedule);
    assert_eq!(join.output_rows().len(), 2);
}

#[test]
fn equi_join_routes_inserts_and_retracts_into_exactly_once_deltas() {
    let schedule = sample();
    let mut join = assignment_shift_join(&schedule);
    assert_eq!(join.output_rows().len(), 2);

    // Retract assignment a0 (descriptor 1, index 0): exactly one output
    // retracts, and the survivor (a2, s0) is untouched.
    let deltas = join.on_retract_source(1, 0);
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].kind, DeltaKind::Retract);
    assert_eq!(deltas[0].left_idx, 0);
    assert_eq!(join.output_rows(), vec![(2, 0)]);

    // Insert it back: exactly one output inserts.
    let deltas = join.on_insert_source(&schedule, 1, 0);
    assert_eq!(deltas.len(), 1);
    assert_eq!(deltas[0].kind, DeltaKind::Insert);
    assert_eq!(deltas[0].left_idx, 0);

    // Unrelated descriptor notifications are exact no-ops.
    assert!(join.on_insert_source(&schedule, 9, 0).is_empty());
    assert!(join.on_retract_source(9, 0).is_empty());
}

#[test]
fn equi_join_right_side_updates_hit_every_matching_output_once() {
    let schedule = sample();
    let mut join = assignment_shift_join(&schedule);

    // Retract shift s0 (descriptor 0, index 0): both (a0, s0) and (a2, s0)
    // retract exactly once each.
    let deltas = join.on_retract_source(0, 0);
    assert_eq!(deltas.len(), 2);
    assert!(deltas.iter().all(|d| d.kind == DeltaKind::Retract));
    assert!(join.output_rows().is_empty());
}
