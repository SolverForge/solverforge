use solverforge::prelude::*;

use super::{Employee, Shift};

#[planning_solution(constraints = "define_constraints", solver_toml = "solver.toml")]
pub struct Schedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

#[solverforge_constraints]
fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    let unassigned = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .filter(|shift: &Shift| shift.employee_idx.is_none())
        .penalize(HardSoftScore::ONE_HARD)
        .named("Unassigned shift");

    let two_shifts_a_day = ConstraintFactory::<Schedule, HardSoftScore>::new()
        .for_each(Schedule::shifts())
        .join((
            ConstraintFactory::<Schedule, HardSoftScore>::new().for_each(Schedule::shifts()),
            |left: &Shift, right: &Shift| {
                left.id < right.id
                    && left.day == right.day
                    && left.employee_idx.is_some()
                    && left.employee_idx == right.employee_idx
            },
        ))
        .penalize(HardSoftScore::ONE_SOFT)
        .named("Two shifts on one day");

    (unassigned, two_shifts_a_day)
}
