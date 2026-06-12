use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "operations",
        route_hooks = "route_hooks",
        savings_hooks = "savings_hooks",
        savings_metric_class_fn = "savings_hooks::metric_class",
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

mod route_hooks {
    pub(super) fn get<S>(_: &S, _: usize) -> Vec<usize> {
        Vec::new()
    }

    pub(super) fn set<S>(_: &mut S, _: usize, _: Vec<usize>) {}

    pub(super) fn depot<S>(_: &S, _: usize) -> usize {
        0
    }

    pub(super) fn distance<S>(_: &S, _: usize, from: usize, to: usize) -> i64 {
        from.abs_diff(to) as i64
    }

    pub(super) fn feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
        true
    }
}

mod savings_hooks {
    pub(super) fn depot<S>(_: &S, _: usize) -> usize {
        0
    }

    pub(super) fn metric_class<S>(_: &S, route_id: usize) -> usize {
        route_id
    }

    pub(super) fn distance<S>(_: &S, _: usize, from: usize, to: usize) -> i64 {
        from.abs_diff(to) as i64
    }

    pub(super) fn feasible<S>(_: &S, _: usize, _: &[usize]) -> bool {
        true
    }
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
