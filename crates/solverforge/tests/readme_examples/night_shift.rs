use super::staffing::{Assignment, Employee, Schedule, Shift};

// readme:begin
use solverforge::prelude::*;
use solverforge::stream::joiner::{equal_bi, equal_on};
use solverforge::stream::relational::{Leaf, operator::Pair};

fn night_shift_staffed() -> impl IncrementalConstraint<Schedule, SoftScore> {
    ConstraintFactory::<Schedule, SoftScore>::new()
        .for_each(Schedule::assignments())
        // Relationship 1 (u32): assignment.shift_id == shift.id
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new().for_each(Schedule::shifts()),
            equal_bi(
                |a: &Assignment| a.shift_id,
                |s: &Shift| s.id,
            ),
        ))
        // Relationship 2 (String): the whole (assignment, shift) row
        // joined to an employee on the employee code.
        .join((
            ConstraintFactory::<Schedule, SoftScore>::new().for_each(Schedule::employees()),
            equal_on(
                |row: &Pair<Leaf<'_, Assignment>, Leaf<'_, Shift>>| {
                    row.left.entity.employee_code.clone()
                },
                |e: &Leaf<'_, Employee>| e.entity.code.clone(),
            ),
        ))
        .filter(|_a: &Assignment, s: &Shift, _e: &Employee| s.night)
        .penalize(SoftScore::of(1))
        .named("night shift staffed")
}
// readme:end

pub fn constraint() -> impl IncrementalConstraint<Schedule, SoftScore> {
    night_shift_staffed()
}
