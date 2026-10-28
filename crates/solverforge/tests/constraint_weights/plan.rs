use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;

use crate::{Task, Worker};

#[planning_solution(
    constraints = "constraints",
    solver_toml = "../fixtures/constraint_weights_solver.toml"
)]
pub struct Plan {
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[constraint_weights]
    pub weights: ConstraintWeights,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

pub const PREFERRED: &str = "Preferred worker";
pub const COST: &str = "Worker cost";

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.worker == Some(task.preferred))
            .reward(HardSoftScore::of_soft(10))
            .named(PREFERRED),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .join((
                Plan::workers(),
                equal_bi(|task: &Task| task.worker, |worker: &Worker| Some(worker.id)),
            ))
            .penalize(|_task: &Task, worker: &Worker| HardSoftScore::of_soft(worker.cost))
            .named(COST),
    )
}
