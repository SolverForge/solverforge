use solverforge::prelude::*;

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "visits", unexpected = "value")]
    pub visits: Vec<usize>,
}

fn main() {}
