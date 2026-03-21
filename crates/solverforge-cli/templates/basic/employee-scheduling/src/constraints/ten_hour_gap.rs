use crate::domain::{EmployeeSchedule, EmployeeScheduleConstraintStreams, Shift};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: At least 10 hours must elapse between consecutive shifts for the same employee.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .shifts()
        .join(joiner::equal(|shift: &Shift| shift.employee_idx))
        .filter(|a: &Shift, b: &Shift| a.id < b.id && a.employee_idx.is_some() && gap_penalty(a, b) > 0)
        .penalize_hard_with(|a: &Shift, b: &Shift| {
            HardSoftDecimalScore::of_hard_scaled(gap_penalty(a, b) * 100_000)
        })
        .named("At least 10 hours between 2 shifts")
}

fn gap_penalty(a: &Shift, b: &Shift) -> i64 {
    const MIN_GAP: i64 = 600; // minutes

    let (earlier, later) = if a.end <= b.start {
        (a, b)
    } else if b.end <= a.start {
        (b, a)
    } else {
        return 0;
    };

    let gap = (later.start - earlier.end).num_minutes();
    if (0..MIN_GAP).contains(&gap) {
        MIN_GAP - gap
    } else {
        0
    }
}
