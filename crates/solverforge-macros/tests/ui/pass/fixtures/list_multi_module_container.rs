use solverforge::prelude::*;

#[planning_entity]
pub struct Container {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "items")]
    pub items: Vec<usize>,
}
