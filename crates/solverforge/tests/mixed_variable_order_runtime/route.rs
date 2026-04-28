use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "visits")]
    pub visits: Vec<usize>,

    #[planning_variable(value_range_provider = "visits", allows_unassigned = true)]
    pub first_visit: Option<usize>,
}
