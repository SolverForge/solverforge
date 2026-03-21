use crate::domain::{Employee, EmployeeSchedule, Shift};
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::IncrementalConstraint;

/// HARD: Every shift must be staffed by an employee with the required skill.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .for_each(shifts)
        .join((
            employees,
            equal_bi(
                |shift: &Shift| shift.employee_idx,
                |emp: &Employee| Some(emp.index),
            ),
        ))
        .filter(|shift: &Shift, emp: &Employee| {
            shift.employee_idx.is_some() && !emp.skills.contains(&shift.required_skill)
        })
        .penalize(HardSoftDecimalScore::ONE_HARD)
        .named("Required skill")
}

fn shifts(schedule: &EmployeeSchedule) -> &[Shift] {
    schedule.shifts.as_slice()
}

fn employees(schedule: &EmployeeSchedule) -> &[Employee] {
    schedule.employees.as_slice()
}
