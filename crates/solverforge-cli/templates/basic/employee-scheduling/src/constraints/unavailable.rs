use crate::domain::{Employee, EmployeeSchedule, Shift};
use chrono::NaiveDate;
use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::IncrementalConstraint;

/// HARD: An employee cannot be assigned to a shift on a day they are unavailable.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
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
            |emp: &Employee| emp.unavailable_days.as_slice(),
            |date: &NaiveDate| *date,
            |shift: &Shift| shift.date(),
        )
        .filter(|shift: &Shift, date: &NaiveDate| {
            shift.employee_idx.is_some() && overlap_minutes(shift, *date) > 0
        })
        .penalize_hard_with(|shift: &Shift, date: &NaiveDate| {
            HardSoftDecimalScore::of_hard_scaled(overlap_minutes(shift, *date) * 100_000)
        })
        .as_constraint("Unavailable employee")
}

fn overlap_minutes(shift: &Shift, date: NaiveDate) -> i64 {
    let day_start = date.and_hms_opt(0, 0, 0).unwrap();
    let day_end = date
        .succ_opt()
        .unwrap_or(date)
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let start = shift.start.max(day_start);
    let end = shift.end.min(day_end);
    if start < end {
        (end - start).num_minutes()
    } else {
        0
    }
}
