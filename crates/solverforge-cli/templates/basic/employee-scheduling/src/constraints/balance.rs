use crate::domain::{EmployeeSchedule, Shift};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: Balance shift assignments evenly across all employees.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .for_each(shifts)
        .balance(|shift: &Shift| shift.employee_idx)
        .penalize(HardSoftDecimalScore::of_soft(1))
        .named("Balance employee assignments")
}

fn shifts(schedule: &EmployeeSchedule) -> &[Shift] {
    schedule.shifts.as_slice()
}
