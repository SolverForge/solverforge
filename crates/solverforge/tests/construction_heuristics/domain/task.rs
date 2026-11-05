use solverforge::prelude::*;

use super::Plan;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    /// Construction difficulty: task 1 is the hardest, task 0 the easiest.
    pub difficulty: i64,

    #[planning_variable(
        value_range_provider = "workers",
        allows_unassigned = true,
        construction_entity_order_key = "task_difficulty",
        construction_value_order_key = "worker_strength"
    )]
    pub worker: Option<usize>,
}

/// A task's difficulty, moved for task 0 only by `difficulty_drift` for
/// every assigned task.
pub(super) fn task_difficulty(plan: &Plan, task: &Task) -> i64 {
    let assigned = plan.tasks.iter().filter(|task| task.worker.is_some()).count() as i64;
    let drift = if task.id == 0 { plan.difficulty_drift * assigned } else { 0 };
    task.difficulty + drift
}

/// A worker's base strength, moved by `load_weight` for every task already
/// on it and, for worker 0 only, by `drift` for every assigned task, so the
/// key changes as construction assigns.
pub(super) fn worker_strength(plan: &Plan, _task: &Task, worker: usize) -> i64 {
    let load = plan
        .tasks
        .iter()
        .filter(|task| task.worker == Some(worker))
        .count() as i64;
    let assigned = plan.tasks.iter().filter(|task| task.worker.is_some()).count() as i64;
    let drift = if worker == 0 { plan.drift * assigned } else { 0 };
    plan.workers[worker].strength + plan.load_weight * load + drift
}
