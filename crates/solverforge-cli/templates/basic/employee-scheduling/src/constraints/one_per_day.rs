use crate::domain::{EmployeeSchedule, EmployeeScheduleConstraintStreams, Shift};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: An employee can work at most one shift per day.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .shifts()
        .join(joiner::equal(|shift: &Shift| (shift.employee_idx, shift.date())))
        .filter(|a: &Shift, b: &Shift| a.id < b.id && a.employee_idx.is_some())
        .penalize(HardSoftDecimalScore::ONE_HARD)
        .named("One shift per day")
}
