use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use super::{Task, Worker};

#[planning_solution(constraints = "constraints")]
pub struct Plan {
    #[problem_fact_collection]
    workers: Vec<Worker>,

    #[planning_entity_collection]
    tasks: Vec<Task>,

    #[planning_score]
    score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::tasks())
        .filter(|task: &Task| task.worker.is_none())
        .penalize(HardSoftScore::of(0, 1))
        .named("unassigned task"),)
}
