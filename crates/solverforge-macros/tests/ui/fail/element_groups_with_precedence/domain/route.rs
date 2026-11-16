use solverforge::prelude::*;

// Group moves do not follow precedence routes, so the hooks exclude each other.
#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "stops",
        precedence_successors_fn = "stop_successors",
        element_group_fn = "stop_group"
    )]
    pub stops: Vec<usize>,
}

pub(super) fn stop_successors(_: &super::Plan, _: usize, _: &mut Vec<usize>) {}

pub(super) fn stop_group(_: &super::Plan, _: usize, _: &mut Vec<usize>) {}
