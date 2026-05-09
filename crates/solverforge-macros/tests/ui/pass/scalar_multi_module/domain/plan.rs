use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Task, Worker};

#[planning_solution(constraints = "constraints")]
pub struct Plan {
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .penalize(HardSoftScore::ONE_SOFT)
            .named("penalize_tasks"),
    )
}
