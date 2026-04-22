use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use crate::{Task, Worker};

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
    use PlanConstraintStreams;

    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .tasks()
            .penalize_soft()
            .named("penalize_tasks"),
    )
}
