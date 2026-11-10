use solverforge::prelude::*;

/// A task with a preferred worker.
#[planning_entity]
pub struct Task {
    /// Stable task id.
    #[planning_id]
    pub id: usize,
    /// Id of the worker this task prefers.
    pub preferred: usize,

    /// Assigned worker.
    #[planning_variable(value_range_provider = "workers")]
    pub worker: Option<usize>,
}
