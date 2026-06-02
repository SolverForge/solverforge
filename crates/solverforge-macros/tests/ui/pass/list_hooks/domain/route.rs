use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "operations",
        element_owner_fn = "operation_owner",
        construction_element_order_key = "operation_construction_order",
        precedence_duration_fn = "operation_duration",
        precedence_successors_fn = "operation_successors"
    )]
    pub operations: Vec<usize>,
}

pub(super) fn operation_owner(plan: &super::Plan, operation_id: usize) -> Option<usize> {
    plan.operations
        .get(operation_id)
        .map(|operation| operation.route_id)
}

pub(super) fn operation_construction_order(plan: &super::Plan, operation_id: usize) -> i64 {
    plan.operations
        .get(operation_id)
        .map_or(0, |operation| operation.duration as i64)
}

pub(super) fn operation_duration(plan: &super::Plan, operation_id: usize) -> usize {
    plan.operations
        .get(operation_id)
        .map_or(0, |operation| operation.duration)
}

pub(super) fn operation_successors(
    plan: &super::Plan,
    operation_id: usize,
    out: &mut Vec<usize>,
) {
    if let Some(next) = plan.operations.get(operation_id).and_then(|operation| operation.next) {
        out.push(next);
    }
}
