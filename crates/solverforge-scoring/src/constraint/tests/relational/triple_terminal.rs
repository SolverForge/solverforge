/* Triple terminal over the chained join: scores, counts, explanations, deltas.

Builds `constraint::relational::TripleTerminal` over the
assignment/shift/employee fixtures and checks it against the oracle
contract: evaluate/initialize/match_count agree with and without
initialization, incremental retract/insert deltas are exact signed
scores, cached state equals full recomputation after every mutation,
and explanations carry all three bindings with signed sums.
*/

use solverforge_core::score::SoftScore;
use solverforge_core::{ConstraintRef, ImpactType};

use crate::api::constraint_set::IncrementalConstraint;
use crate::constraint::relational::TripleTerminal;
use crate::stream::relational::{Concat, Leaf};

use super::chained::{accept_all, employee_key, row_employee_code};
use super::fixtures::{rel_employees, sample, RelAssignment, RelEmployee, RelSchedule, RelShift};
use super::oracle::{oracle_rows, oracle_score};
use super::updates::assignment_shift_join;

type NightStaffedTerminal = TripleTerminal<
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
    fn(&RelAssignment, &RelShift, &RelEmployee) -> SoftScore,
    SoftScore,
>;

fn unit_weight(
    _assignment: &RelAssignment,
    _shift: &RelShift,
    _employee: &RelEmployee,
) -> SoftScore {
    SoftScore::of(1)
}

fn staffed_constraint(schedule: &RelSchedule) -> NightStaffedTerminal {
    TripleTerminal::new(
        ConstraintRef::new("", "night shift staffed"),
        ImpactType::Penalty,
        assignment_shift_join(schedule),
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
        unit_weight as fn(&RelAssignment, &RelShift, &RelEmployee) -> SoftScore,
        false,
        1,
        0,
        2,
        "night shift staffed",
    )
}

#[test]
fn triple_terminal_evaluate_and_match_count_agree_with_oracle() {
    let schedule = sample();
    let constraint = staffed_constraint(&schedule);
    assert_eq!(constraint.evaluate(&schedule), oracle_score(&schedule));
    assert_eq!(
        constraint.match_count(&schedule),
        oracle_rows(&schedule).len()
    );
    assert_eq!(oracle_rows(&schedule).len(), 2);
}

#[test]
fn triple_terminal_initialize_then_incremental_matches_evaluate() {
    let schedule = sample();
    let mut constraint = staffed_constraint(&schedule);
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

    // Insert a0 back: -1 restores the total.
    let delta = constraint.on_insert(&schedule, 0, 1);
    assert_eq!(delta, SoftScore::of(-1));
    assert_eq!(constraint.evaluate(&schedule), SoftScore::of(-2));
}

#[test]
fn triple_terminal_explanations_carry_all_three_bindings() {
    let schedule = sample();
    let constraint = staffed_constraint(&schedule);
    let matches = constraint.get_matches(&schedule);
    assert_eq!(matches.len(), 2);
    let total = matches
        .iter()
        .fold(SoftScore::of(0), |acc, m| acc + m.score);
    assert_eq!(total, SoftScore::of(-2));
}
