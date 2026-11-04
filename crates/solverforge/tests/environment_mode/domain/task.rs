use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    pub weight: i64,

    #[planning_variable(value_range_provider = "resources")]
    pub resource_idx: Option<usize>,
}
