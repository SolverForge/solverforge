use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    /// Multiplies the worker's cost.
    pub weight: i64,

    #[planning_variable(value_range_provider = "workers")]
    pub worker: Option<usize>,
}
