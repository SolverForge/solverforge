use solverforge::prelude::*;
use solverforge::stream::ConstraintFactory;

use crate::{Task, WorkTask, Worker};

/// Workers and tasks for the multi-module scalar hook test.
#[planning_solution(
    constraints = "constraints",
    solver_toml = "../fixtures/scalar_multi_module_runtime_solver.toml"
)]
pub struct Plan {
    /// Workers tasks can be assigned to.
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    /// Tasks the solver assigns.
    #[planning_entity_collection]
    pub tasks: Vec<WorkTask>,

    /// The plan's score.
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
