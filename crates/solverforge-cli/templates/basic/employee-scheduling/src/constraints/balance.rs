use crate::domain::{EmployeeSchedule, Shift};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// SOFT: Balance shift assignments evenly across all employees.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .balance(|shift: &Shift| shift.employee_idx)
        .penalize(HardSoftDecimalScore::of_soft(1))
        .as_constraint("Balance employee assignments")
}
