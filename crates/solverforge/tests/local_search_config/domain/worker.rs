use solverforge::prelude::*;

#[problem_fact]
pub struct Worker {
    #[planning_id]
    pub id: usize,

    /// Soft cost of a task on this worker, per unit of task weight.
    pub cost: i64,
}
