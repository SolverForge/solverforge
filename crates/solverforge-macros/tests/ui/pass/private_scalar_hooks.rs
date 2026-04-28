#[path = "private_scalar_hooks/domain/mod.rs"]
mod domain;

use domain::Plan;

fn main() {
    let descriptor = Plan::descriptor();
    let task_descriptor = descriptor
        .find_entity_descriptor("Task")
        .expect("task descriptor should exist");
    let worker_variable = task_descriptor
        .find_variable("worker")
        .expect("worker variable should exist");

    let _ = (
        worker_variable.nearby_value_distance_meter.is_some(),
        worker_variable.nearby_entity_distance_meter.is_some(),
        worker_variable.candidate_values.is_some(),
        worker_variable.nearby_value_candidates.is_some(),
        worker_variable.nearby_entity_candidates.is_some(),
        worker_variable.construction_entity_order_key.is_some(),
        worker_variable.construction_value_order_key.is_some(),
    );
}
