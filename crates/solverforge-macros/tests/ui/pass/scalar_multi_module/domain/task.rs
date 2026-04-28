use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    #[planning_variable(value_range_provider = "workers", allows_unassigned = true)]
    pub worker: Option<usize>,
}
