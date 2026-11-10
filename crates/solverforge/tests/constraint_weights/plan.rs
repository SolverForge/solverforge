use solverforge::prelude::*;
use solverforge::stream::joiner::equal_bi;
use solverforge::stream::ConstraintFactory;

use crate::{Task, Worker};

/// Workers, tasks and per-constraint weights for the weighting tests.
#[planning_solution(
    constraints = "constraints",
    solver_toml = "../fixtures/constraint_weights_solver.toml"
)]
pub struct Plan {
    /// Workers tasks can be assigned to.
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    /// Tasks the solver assigns.
    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    /// Weight multiplier per constraint name.
    #[constraint_weights]
    pub weights: ConstraintWeights,

    /// The plan's score.
    #[planning_score]
    pub score: Option<HardSoftScore>,
}

/// Name of the constraint that rewards a task's preferred worker.
pub const PREFERRED: &str = "Preferred worker";
/// Name of the constraint that penalizes each worker's cost.
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
