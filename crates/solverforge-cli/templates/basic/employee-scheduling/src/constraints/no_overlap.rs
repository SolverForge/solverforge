use crate::domain::{EmployeeSchedule, EmployeeScheduleConstraintStreams, Shift};
use solverforge::prelude::*;
use solverforge::IncrementalConstraint;

/// HARD: An employee cannot work two overlapping shifts.
pub fn constraint() -> impl IncrementalConstraint<EmployeeSchedule, HardSoftDecimalScore> {
    ConstraintFactory::<EmployeeSchedule, HardSoftDecimalScore>::new()
        .shifts()
        .join(joiner::equal(|shift: &Shift| shift.employee_idx))
        .filter(|a: &Shift, b: &Shift| {
            a.id < b.id && a.employee_idx.is_some() && a.start < b.end && b.start < a.end
        })
        .penalize_hard_with(|a: &Shift, b: &Shift| {
            HardSoftDecimalScore::of_hard_scaled(overlap_minutes(a, b) * 100_000)
        })
        .named("Overlapping shift")
}

fn overlap_minutes(a: &Shift, b: &Shift) -> i64 {
    let start = a.start.max(b.start);
    let end = a.end.min(b.end);
    if start < end {
        (end - start).num_minutes()
    } else {
        0
    }
}
