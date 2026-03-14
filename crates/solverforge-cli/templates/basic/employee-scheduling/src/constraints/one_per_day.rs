use crate::domain::{EmployeeSchedule, Shift};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: An employee can work at most one shift per day.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .for_each_unique_pair(
            |s: &EmployeeSchedule| s.shifts.as_slice(),
            joiner::equal(|shift: &Shift| (shift.employee_idx, shift.date())),
        )
        .filter(|a: &Shift, b: &Shift| a.employee_idx.is_some() && b.employee_idx.is_some())
        .penalize(HardSoftDecimalScore::ONE_HARD)
        .as_constraint("One shift per day")
}
