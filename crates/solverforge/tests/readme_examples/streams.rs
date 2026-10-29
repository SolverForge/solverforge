use solverforge::prelude::*;

use super::nurses::{Schedule, Shift};

pub fn long_work_streaks() {
// readme:begin
ConstraintFactory::<Schedule, HardSoftScore>::new()
    .for_each(Schedule::shifts())
    .filter(|shift: &Shift| shift.nurse_idx.is_some())
    .group_by(
        |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
        consecutive_runs(|shift: &Shift| shift.day),
    )
    .penalize(|_nurse_idx: &usize, runs: &Runs| {
        let excess_days = runs
            .runs()
            .iter()
            .map(|run| run.point_count().saturating_sub(2) as i64)
            .sum();
        HardSoftScore::of_soft(excess_days)
    })
    .named("Long work streaks");
// readme:end
}
