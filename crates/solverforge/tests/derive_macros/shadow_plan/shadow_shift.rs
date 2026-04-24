use solverforge::prelude::*;

#[planning_entity]
pub struct ShadowShift {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "shift_visits")]
    pub visits: Vec<usize>,
}
