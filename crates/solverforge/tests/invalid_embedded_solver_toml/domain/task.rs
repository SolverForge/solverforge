use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: String,

    #[planning_variable(value_range_provider = "resources", allows_unassigned = true)]
    pub resource_idx: Option<usize>,
}
