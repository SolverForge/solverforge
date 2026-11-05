use solverforge::prelude::*;
use solverforge::stream::{joiner::equal, ConstraintFactory};
use solverforge::SolverConfig;

use super::{Task, Worker};

#[planning_solution(constraints = "constraints", config = "solver_config")]
pub struct Plan {
    #[problem_fact_collection]
    pub workers: Vec<Worker>,

    #[planning_entity_collection]
    pub tasks: Vec<Task>,

    #[planning_score]
    pub score: Option<HardSoftScore>,

    /// How much each assigned task moves its worker's strength key.
    pub load_weight: i64,

    /// How much each assigned task moves worker 0's strength key.
    pub drift: i64,

    /// How much each assigned task moves task 0's difficulty key.
    pub difficulty_drift: i64,

    pub heuristic: &'static str,
}

impl Plan {
    /// Three tasks with difficulties 1, 3, 2 and three workers with
    /// strengths 2, 0, 1 and costs 3, 2, 1.
    pub fn new(heuristic: &'static str, load_weight: i64) -> Self {
        Self {
            workers: [2, 0, 1]
                .into_iter()
                .enumerate()
                .map(|(id, strength)| Worker {
                    id,
                    strength,
                    cost: 3 - id as i64,
                })
                .collect(),
            tasks: [1, 3, 2]
                .into_iter()
                .enumerate()
                .map(|(id, difficulty)| Task {
                    id,
                    difficulty,
                    worker: None,
                })
                .collect(),
            score: None,
            load_weight,
            drift: 0,
            difficulty_drift: 0,
            heuristic,
        }
    }

    pub fn workers_by_task(&self) -> Vec<Option<usize>> {
        self.tasks.iter().map(|task| task.worker).collect()
    }
}

fn solver_config(plan: &Plan, _base: SolverConfig) -> SolverConfig {
    SolverConfig::from_toml_str(&format!(
        "[[phases]]\ntype = \"construction_heuristic\"\nconstruction_heuristic_type = \"{}\"\n",
        plan.heuristic
    ))
    .expect("the test config parses")
}

/// An unassigned task costs one hard point and two tasks sharing a worker
/// two, so a task goes only to a free worker; each assigned task costs its
/// worker's soft cost.
fn constraints() -> impl ConstraintSet<Plan, HardSoftScore> {
    (
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .filter(|task: &Task| task.worker.is_none())
            .penalize(HardSoftScore::ONE_HARD)
            .named("unassigned task"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .join(equal(|task: &Task| task.worker))
            .penalize(|left: &Task, _right: &Task| {
                HardSoftScore::of(if left.worker.is_some() { 2 } else { 0 }, 0)
            })
            .named("shared worker"),
        ConstraintFactory::<Plan, HardSoftScore>::new()
            .for_each(Plan::tasks())
            .join((
                Plan::workers(),
                solverforge::stream::joiner::equal_bi(
                    |task: &Task| task.worker,
                    |worker: &Worker| Some(worker.id),
                ),
            ))
            .penalize(|_task: &Task, worker: &Worker| HardSoftScore::of_soft(worker.cost))
            .named("worker cost"),
    )
}
