use solverforge::prelude::*;

use super::Shift;

// readme:begin
#[planning_solution(constraints = "define_constraints")]
pub struct Schedule {
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[constraint_weights]
    pub weights: ConstraintWeights,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
// readme:end

fn define_constraints() -> impl ConstraintSet<Schedule, HardSoftScore> {
    (
        ConstraintFactory::<Schedule, HardSoftScore>::new()
            .for_each(Schedule::shifts())
            .penalize(|shift: &Shift| HardSoftScore::of_soft(shift.minutes))
            .named("Travel time"),
        ConstraintFactory::<Schedule, HardSoftScore>::new()
            .for_each(Schedule::shifts())
            .filter(|shift: &Shift| shift.minutes > 480)
            .penalize(HardSoftScore::ONE_SOFT)
            .named("Overtime"),
    )
}
