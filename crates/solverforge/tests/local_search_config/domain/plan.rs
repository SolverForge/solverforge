use solverforge::prelude::*;
use solverforge::stream::joiner::{equal, equal_bi};
use solverforge::stream::ConstraintFactory;

use super::{Task, Worker};

#[planning_solution(constraints = "constraints")]
pub struct Plan {
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<SoftScore>,
}

impl Plan {
    /// One unassigned task per entry of `weights` and one worker per entry
    /// of `costs`.
    pub fn new(weights: &[i64], costs: &[i64]) -> Self {
        Self {
            workers: costs
                .iter()
                .enumerate()
                .map(|(id, &cost)| Worker { id, cost })
                .collect(),
            tasks: weights
                .iter()
                .enumerate()
                .map(|(id, &weight)| Task {
                    id,
                    weight,
                    worker: None,
                })
                .collect(),
            score: None,
        }
    }

    pub fn workers_by_task(&self) -> Vec<Option<usize>> {
        self.tasks.iter().map(|task| task.worker).collect()
    }
}

/// Each task costs its weight times its worker's cost; two tasks sharing a
/// worker cost 10.
fn constraints() -> impl ConstraintSet<Plan, SoftScore> {
    (
        ConstraintFactory::<Plan, SoftScore>::new()
            .for_each(Plan::tasks())
            .join(equal(|task: &Task| task.worker))
            .penalize(|left: &Task, _right: &Task| {
                SoftScore::of(if left.worker.is_some() { 10 } else { 0 })
            })
            .named("shared worker"),
        ConstraintFactory::<Plan, SoftScore>::new()
            .for_each(Plan::tasks())
            .join((
                Plan::workers(),
                equal_bi(|task: &Task| task.worker, |worker: &Worker| Some(worker.id)),
            ))
            .penalize(|task: &Task, worker: &Worker| SoftScore::of(task.weight * worker.cost))
            .named("worker cost"),
    )
}
