use solverforge::prelude::*;

use super::Plan;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    #[planning_variable(
        value_range = "workers",
        allows_unassigned = true,
        candidate_values = "worker_candidates",
        nearby_value_candidates = "nearby_worker_candidates",
        nearby_entity_candidates = "nearby_task_candidates",
        nearby_value_distance_meter = "worker_value_distance",
        nearby_entity_distance_meter = "task_distance",
        construction_entity_order_key = "task_priority",
        construction_value_order_key = "worker_priority"
    )]
    pub worker: Option<usize>,
}

pub(super) fn worker_value_distance(_plan: &Plan, task: &Task, worker: usize) -> f64 {
    task.id.abs_diff(worker) as f64
}

pub(super) fn worker_candidates(_plan: &Plan, _entity_index: usize, _variable_index: usize) -> &[usize] {
    &[0, 1, 2]
}

pub(super) fn nearby_worker_candidates(
    _plan: &Plan,
    _entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &[1, 2]
}

pub(super) fn nearby_task_candidates(
    _plan: &Plan,
    _entity_index: usize,
    _variable_index: usize,
) -> &[usize] {
    &[1]
}

pub(super) fn task_distance(_plan: &Plan, left: &Task, right: &Task) -> f64 {
    left.id.abs_diff(right.id) as f64
}

pub(super) fn task_priority(_plan: &Plan, task: &Task) -> i64 {
    -(task.id as i64)
}

pub(super) fn worker_priority(_plan: &Plan, task: &Task, worker: usize) -> i64 {
    task.id.abs_diff(worker) as i64
}
