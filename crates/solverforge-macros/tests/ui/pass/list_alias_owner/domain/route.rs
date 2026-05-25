use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(
        element_collection = "visits",
        element_owner_fn = "route_visit_owner"
    )]
    pub visits: Vec<usize>,
}

pub(super) fn route_visit_owner(_: &super::Plan, visit_id: usize) -> Option<usize> {
    Some(visit_id)
}
