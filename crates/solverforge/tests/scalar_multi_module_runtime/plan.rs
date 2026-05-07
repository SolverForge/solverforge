use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use crate::{Task, WorkTask, Worker};

#[planning_solution(
    constraints = "constraints",
    solver_toml = "../fixtures/scalar_multi_module_runtime_solver.toml"
)]
pub struct Plan {
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    #[planning_entity_collection]
    pub tasks: Vec<WorkTask>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (ConstraintFactory::<Plan, HardSoftScore>::new()
        .for_each(Plan::tasks())
        .filter(|task: &Task| task.worker.is_none())
        .penalize(HardSoftScore::of(0, 1))
        .named("unassigned task"),)
}
