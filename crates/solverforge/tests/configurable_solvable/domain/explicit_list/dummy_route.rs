use solverforge::prelude::*;

#[planning_entity]
pub struct DummyRoute {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "visits")]
    pub visits: Vec<usize>,
}
