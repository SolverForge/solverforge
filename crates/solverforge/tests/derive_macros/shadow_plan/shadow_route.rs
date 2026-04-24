use solverforge::prelude::*;

#[planning_entity]
pub struct ShadowRoute {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "routed_visits")]
    pub visits: Vec<usize>,
}
