use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,
    pub preferred: usize,

    #[planning_variable(value_range_provider = "workers")]
    pub worker: Option<usize>,
}
