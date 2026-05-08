use solverforge::prelude::*;

use super::{Nurse, Shift};

#[planning_solution(
    constraints = "define_constraints",
    solver_toml = "../../solver.toml",
    scalar_groups = "scalar_groups"
)]
pub struct Schedule {
    #[problem_fact_collection]
    pub nurses: Vec<Nurse>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let unassigned_required = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .filter(|shift: &Shift| shift.required && shift.nurse_idx.is_none())
        .penalize_hard()
        .named("Unassigned required shift");

    let one_shift_per_nurse_day = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .join((
            ConstraintFactory::<Schedule, HardSoftScore>::new().for_each(Schedule::shifts()),
            |left: &Shift, right: &Shift| {
                left.id < right.id
                    && left.day == right.day
                    && left.nurse_idx.is_some()
                    && left.nurse_idx == right.nurse_idx
            },
        ))
        .penalize_hard()
        .named("One shift per nurse day");

    let long_work_streaks = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .filter(|shift: &Shift| shift.nurse_idx.is_some())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            consecutive_runs(|shift: &Shift| shift.day),
        )
        .penalize_with(|_nurse_idx: &usize, runs: &Runs| {
            let excess_days = runs
                .runs()
                .iter()
                .map(|run| run.point_count().saturating_sub(2) as i64)
                .sum();
            HardSoftScore::of_soft(excess_days)
        })
        .named("Long work streaks");

    let balanced_workload = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .filter(|shift: &Shift| shift.nurse_idx.is_some())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            count::<Shift>(),
        )
        .complement(Schedule::nurses(), |nurse: &Nurse| nurse.id, |_nurse| 0usize)
        .penalize_with(|_nurse_idx: &usize, count: &usize| {
            let target = 4i64;
            HardSoftScore::of_soft((*count as i64 - target).abs())
        })
        .named("Balanced workload");

    (
        unassigned_required,
        one_shift_per_nurse_day,
        long_work_streaks,
        balanced_workload,
    )
}

pub(super) fn scalar_groups() -> Vec<ScalarGroup<Schedule>> {
    vec![
        ScalarGroup::assignment(
            "required_shift_assignment",
            Schedule::shifts().scalar("nurse_idx"),
        )
        .with_required_entity(required_shift)
        .with_capacity_key(nurse_day_capacity)
        .with_entity_order(shift_order)
        .with_value_order(nurse_preference),
    ]
}

fn required_shift(schedule: &Schedule, shift_idx: usize) -> bool {
    schedule.shifts[shift_idx].required
}

fn nurse_day_capacity(schedule: &Schedule, shift_idx: usize, nurse_idx: usize) -> Option<usize> {
    let shift = &schedule.shifts[shift_idx];
    usize::try_from(shift.day)
        .ok()
        .and_then(|day| day.checked_mul(schedule.nurses.len()))
        .and_then(|base| base.checked_add(nurse_idx))
}

fn shift_order(schedule: &Schedule, shift_idx: usize) -> i64 {
    let shift = &schedule.shifts[shift_idx];
    let slot = match i64::try_from(shift.slot) {
        Ok(slot) => slot,
        Err(_) => return i64::MAX,
    };
    shift.day.saturating_mul(10).saturating_add(slot)
}

fn nurse_preference(schedule: &Schedule, shift_idx: usize, nurse_idx: usize) -> i64 {
    let shift = &schedule.shifts[shift_idx];
    let nurse_count = schedule.nurses.len();
    let Ok(day) = usize::try_from(shift.day) else {
        return i64::MAX;
    };
    let preferred = (day + shift.slot) % nurse_count;
    let distance = (nurse_idx + nurse_count - preferred) % nurse_count;
    match i64::try_from(distance) {
        Ok(distance) => distance,
        Err(_) => i64::MAX,
    }
}
