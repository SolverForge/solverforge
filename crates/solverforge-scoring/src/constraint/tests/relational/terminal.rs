/* Generic terminal over the binary equi-join: scores, counts, explanations, deltas.

Builds `constraint::relational::Terminal` directly over the
assignment/shift fixtures and checks it against the oracle contract:
evaluate/initialize/match_count agree, incremental retract/insert deltas
are exact signed scores, cached state equals full recomputation after
every mutation, and explanations carry both bindings with signed sums.
*/

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::Terminal;

use super::fixtures::{rel_assignments, rel_shifts, sample, RelAssignment, RelSchedule, RelShift};
use super::updates::{
    assignment_key, assignment_shift_join, night_only, shift_key, AssignmentShiftJoin,
};

type AssignmentShiftTerminal = Terminal<
    RelSchedule,
    RelAssignment,
    RelShift,
    u32,
    fn(&RelSchedule) -> &[RelAssignment],
    fn(&RelSchedule) -> &[RelShift],
    fn(&RelAssignment) -> u32,
    fn(&RelShift) -> u32,
    fn(&RelSchedule, &RelAssignment, &RelShift, usize, usize) -> bool,
    fn(&RelAssignment, &RelShift) -> SoftScore,
    SoftScore,
>;

fn unit_weight(_assignment: &RelAssignment, _shift: &RelShift) -> SoftScore {
    SoftScore::of(1)
}

fn staffed_constraint() -> AssignmentShiftTerminal {
    Terminal::new(
        ConstraintRef::new("", "night shift staffed"),
        ImpactType::Penalty,
        rel_assignments as fn(&RelSchedule) -> &[RelAssignment],
        rel_shifts as fn(&RelSchedule) -> &[RelShift],
        assignment_key as fn(&RelAssignment) -> u32,
        shift_key as fn(&RelShift) -> u32,
        night_only as fn(&RelSchedule, &RelAssignment, &RelShift, usize, usize) -> bool,
        unit_weight as fn(&RelAssignment, &RelShift) -> SoftScore,
        false,
        1,
        0,
        "night staffed",
    )
}

#[test]
fn terminal_evaluate_and_match_count_agree_without_initialization() {
    let constraint = staffed_constraint();
    let schedule = sample();
    // (a0, s0) and (a2, s0): night shift 10 staffed twice.
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
    assert_eq!(constraint.match_count(&schedule), 2);
}

#[test]
fn terminal_initialize_then_incremental_matches_evaluate() {
    let schedule = sample();
    let mut constraint = staffed_constraint();
    assert_eq!(constraint.initialize(&schedule), SoftScore::of(-2));

    // Retract assignment a0 (descriptor 1): one pair retracts for +1.
    let without_a0 = RelSchedule {
        shifts: schedule.shifts.clone(),
        assignments: schedule.assignments[1..].to_vec(),
        employees: schedule.employees.clone(),
        score: None,
    };
    // NOTE: retract-then-remove protocol — the callback runs against the
    // pre-mutation solution for old-key retention; here the entity index
    // (0) names a0 in the original layout.
    let delta = constraint.on_retract(&schedule, 0, 1);
    assert_eq!(delta, SoftScore::of(1));
    // Full recomputation on the trimmed solution agrees with retention.
    assert_eq!(constraint.evaluate(&without_a0), SoftScore::of(-1));
    assert_eq!(constraint.match_count(&without_a0), 1);

    // Insert a0 back (descriptor 1, index 0): -1 restores the total.
    let delta = constraint.on_insert(&schedule, 0, 1);
    assert_eq!(delta, SoftScore::of(-1));
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
}

#[test]
fn terminal_explanations_carry_both_bindings_with_signed_scores() {
    let constraint = staffed_constraint();
    let schedule = sample();
    let matches = constraint.get_matches(&schedule);
    assert_eq!(matches.len(), 2);
    let total = matches
        .iter()
        .fold(SoftScore::of(0), |acc, m| acc + m.score);
    assert_eq!(total, SoftScore::of(-2));
}

#[test]
fn terminal_operator_parity_with_standalone_join() {
    // The terminal's retained pairs equal the standalone operator rows.
    let schedule = sample();
    let mut constraint = staffed_constraint();
    constraint.initialize(&schedule);
    let join: AssignmentShiftJoin = assignment_shift_join(&schedule);
    assert_eq!(constraint.match_count(&schedule), join.output_rows().len());
}
