use crate::domain::{Employee, EmployeeSchedule, Shift};
use chrono::NaiveDate;
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::IncrementalConstraint;

/// SOFT: Penalize shifts on days an employee marked as undesired.
pub fn undesired_constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore>
{
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .flatten_last(
            |emp: &Employee| emp.undesired_days.as_slice(),
            |date: &NaiveDate| *date,
            |shift: &Shift| shift.date(),
        )
        .filter(|shift: &Shift, _date: &NaiveDate| shift.employee_idx.is_some())
        .penalize(HardSoftDecimalScore::ONE_SOFT)
        .as_constraint("Undesired day for employee")
}

/// SOFT: Reward shifts on days an employee marked as desired.
pub fn desired_constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .for_each(|s: &EmployeeSchedule| s.shifts.as_slice())
        .join(
            |s: &EmployeeSchedule| s.employees.as_slice(),
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        )
        .flatten_last(
            |emp: &Employee| emp.desired_days.as_slice(),
            |date: &NaiveDate| *date,
            |shift: &Shift| shift.date(),
        )
        .filter(|shift: &Shift, _date: &NaiveDate| shift.employee_idx.is_some())
        .reward(HardSoftDecimalScore::ONE_SOFT)
        .as_constraint("Desired day for employee")
}
