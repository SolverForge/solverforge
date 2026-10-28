// Weight multipliers on every constraint family, checked against an oracle.

use super::support::*;
use crate::constraint::{BalanceConstraint, ListPrecedenceMakespanConstraint};
use crate::stream::collector::{count, sum};
use crate::stream::filter::TrueFilter;
use crate::stream::joiner::{equal, equal_bi};
use crate::stream::{Projection, ProjectionSink};
use solverforge_core::{ConstraintRef, ImpactType};

#[derive(Clone, Debug)]
struct HoursRow {
    employee: usize,
    hours: i64,
    mentor: Option<usize>,
}

struct AssignedHours;

impl Projection<Shift> for AssignedHours {
    type Out = HoursRow;
    const MAX_EMITS: usize = 1;

    fn project<Sink>(&self, shift: &Shift, out: &mut Sink)
    where
        Sink: ProjectionSink<Self::Out>,
    {
        if let Some(employee) = shift.employee {
            out.emit(HoursRow {
                employee,
                hours: shift.hours,
                mentor: shift.mentor,
            });
        }
    }
}

fn shift_source() -> impl crate::stream::collection_extract::CollectionExtract<Plan, Item = Shift> {
    source(shifts as fn(&Plan) -> &[Shift], SHIFTS)
}

fn employee_source(
) -> impl crate::stream::collection_extract::CollectionExtract<Plan, Item = Employee> {
    source(employees as fn(&Plan) -> &[Employee], EMPLOYEES)
}

fn factory() -> ConstraintFactory<Plan, HardSoftScore> {
    ConstraintFactory::new()
}

#[test]
fn uni_constraint_scales_fixed_and_closure_weights() {
    assert_family("Unassigned", || {
        factory()
            .for_each(shift_source())
            .filter(|shift: &Shift| shift.employee.is_none())
            .penalize(HardSoftScore::of_hard(1))
            .named("Unassigned")
    });
    assert_family("Hours", || {
        factory()
            .for_each(shift_source())
            .reward(|shift: &Shift| HardSoftScore::of_soft(shift.hours))
            .named("Hours")
    });
}

#[test]
fn join_terminal_scales_every_pair() {
    assert_family("Over capacity", || {
        factory()
            .for_each(shift_source())
            .join((
                employee_source(),
                equal_bi(
                    |shift: &Shift| shift.employee,
                    |employee: &Employee| Some(employee.id),
                ),
            ))
            .filter(|shift: &Shift, employee: &Employee| shift.hours > employee.capacity / 2)
            .penalize(|shift: &Shift, employee: &Employee| {
                HardSoftScore::of_soft(shift.hours - employee.capacity / 2)
            })
            .named("Over capacity")
    });
}

#[test]
fn self_join_terminal_scales_every_pair() {
    assert_family("Shared employee", || {
        factory()
            .for_each(shift_source())
            .join(equal(|shift: &Shift| shift.employee))
            .filter(|left: &Shift, _right: &Shift| left.employee.is_some())
            .penalize(|left: &Shift, right: &Shift| {
                HardSoftScore::of_soft(left.hours + right.hours)
            })
            .named("Shared employee")
    });
}

#[test]
fn existence_terminal_scales_every_row() {
    assert_family("Idle employee", || {
        factory()
            .for_each(employee_source())
            .if_not_exists((
                factory().for_each(shift_source()),
                equal_bi(
                    |employee: &Employee| Some(employee.id),
                    |shift: &Shift| shift.employee,
                ),
            ))
            .penalize(|employee: &Employee| HardSoftScore::of_soft(employee.capacity))
            .named("Idle employee")
    });
}

#[test]
fn grouped_terminal_scales_every_group() {
    assert_family("Workload", || {
        factory()
            .for_each(shift_source())
            .group_by(
                |shift: &Shift| shift.employee.unwrap_or(usize::MAX),
                sum(|shift: &Shift| shift.hours),
            )
            .penalize(|_employee: &usize, hours: &i64| HardSoftScore::of_soft(hours * hours))
            .named("Workload")
    });
}

#[test]
fn complemented_terminal_scales_every_target() {
    assert_family("Shift count", || {
        factory()
            .for_each(shift_source())
            .group_by(
                |shift: &Shift| shift.employee.unwrap_or(usize::MAX),
                count(),
            )
            .complement(employee_source(), |e: &Employee| e.id, |_e| 0usize)
            .penalize(|_employee: &usize, count: &usize| {
                HardSoftScore::of_soft((*count as i64 - 2).abs())
            })
            .named("Shift count")
    });
}

#[test]
fn projected_uni_scales_every_output() {
    assert_family("Projected hours", || {
        factory()
            .for_each(shift_source())
            .project(AssignedHours)
            .penalize(|row: &HoursRow| HardSoftScore::of_soft(row.hours))
            .named("Projected hours")
    });
}

#[test]
fn projected_bi_scales_every_pair() {
    assert_family("Projected pair", || {
        factory()
            .for_each(shift_source())
            .project(AssignedHours)
            .join(equal(|row: &HoursRow| row.employee))
            .penalize(|left: &HoursRow, right: &HoursRow| {
                HardSoftScore::of_soft(left.hours * right.hours)
            })
            .named("Projected pair")
    });
}

#[test]
fn projected_directed_bi_scales_every_pair() {
    assert_family("Mentor load", || {
        factory()
            .for_each(shift_source())
            .project(AssignedHours)
            .join(equal_bi(
                |left: &HoursRow| Some(left.employee),
                |right: &HoursRow| right.mentor,
            ))
            .penalize(|left: &HoursRow, right: &HoursRow| {
                HardSoftScore::of_soft(left.hours + 2 * right.hours)
            })
            .named("Mentor load")
    });
}

#[test]
fn balance_scales_after_standard_deviation() {
    assert_family("Balance", || {
        BalanceConstraint::new(
            ConstraintRef::new("", "Balance"),
            ImpactType::Penalty,
            shift_source(),
            TrueFilter,
            |shift: &Shift| shift.employee,
            HardSoftScore::of_soft(1_000),
            false,
        )
    });
}

fn node_count(plan: &Plan) -> usize {
    plan.tasks.len()
}

fn node_duration(plan: &Plan, node: usize) -> usize {
    plan.tasks[node].duration
}

fn fixed_successors(plan: &Plan, node: usize, out: &mut Vec<usize>) {
    out.extend(plan.tasks[node].next);
}

fn owner_count(plan: &Plan) -> usize {
    plan.routes.len()
}

fn list_len(plan: &Plan, owner: usize) -> usize {
    plan.routes[owner].tasks.len()
}

fn list_get(plan: &Plan, owner: usize, pos: usize) -> Option<usize> {
    plan.routes[owner].tasks.get(pos).copied()
}

#[test]
fn list_precedence_scales_hard_and_makespan_levels() {
    assert_family("Makespan", || {
        ListPrecedenceMakespanConstraint::new(
            ConstraintRef::new("", "Makespan"),
            ROUTES,
            node_count,
            node_duration,
            fixed_successors,
            owner_count,
            list_len,
            list_get,
        )
    });
}
