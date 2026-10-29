use solverforge::prelude::*;
use solverforge::SolverConfig;

use super::{Nurse, Shift};

// readme:begin
#[planning_solution(
    constraints = "define_constraints",
    config = "solver_config_for_solution"
)]
pub struct Schedule {
    #[problem_fact_collection]
    pub nurses: Vec<Nurse>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    pub time_limit_secs: u64,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn solver_config_for_solution(solution: &Schedule, config: SolverConfig) -> SolverConfig {
    config.with_termination_seconds(solution.time_limit_secs)
}
// readme:end

// readme:begin
#[solverforge_constraints]
fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let g = ConstraintFactory::<Schedule, HardSoftScore>::new();
    let nurse_presence = g
        .for_each(Schedule::shifts())
        .filter(|shift: &Shift| shift.nurse_idx.is_some())
        .group_by(
            |shift: &Shift| shift.nurse_idx.unwrap_or(usize::MAX),
            indexed_presence(|shift: &Shift| shift.day),
        );

    (
        nurse_presence
            .penalize(consecutive_work_bounds)
            .named("consecutiveWorkBounds"),
        nurse_presence
            .penalize(consecutive_off_bounds)
            .named("consecutiveOffBounds"),
        nurse_presence
            .penalize(working_weekends)
            .named("workingWeekends"),
    )
}
// readme:end

fn consecutive_work_bounds(_nurse_idx: &usize, presence: &IndexedPresence) -> HardSoftScore {
    let excess_days = presence
        .runs()
        .runs()
        .iter()
        .map(|run| run.point_count().saturating_sub(5) as i64)
        .sum();
    HardSoftScore::of_soft(excess_days)
}

fn consecutive_off_bounds(_nurse_idx: &usize, presence: &IndexedPresence) -> HardSoftScore {
    let excess_days = presence
        .complement_runs(0..28)
        .runs()
        .iter()
        .map(|run| run.point_count().saturating_sub(3) as i64)
        .sum();
    HardSoftScore::of_soft(excess_days)
}

fn working_weekends(_nurse_idx: &usize, presence: &IndexedPresence) -> HardSoftScore {
    let weekends = (0..4)
        .filter(|week| presence.any_in(week * 7 + 5..week * 7 + 7))
        .count();
    HardSoftScore::of_soft(weekends as i64)
}
