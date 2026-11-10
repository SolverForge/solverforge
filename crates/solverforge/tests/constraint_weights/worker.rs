use solverforge::prelude::*;

/// A worker with a cost.
#[problem_fact]
pub struct Worker {
    /// Stable worker id.
    #[planning_id]
    pub id: usize,
    /// Cost of assigning a task to this worker.
    pub cost: i64,
}
