use solverforge::prelude::*;

#[problem_fact]
pub struct Worker {
    #[planning_id]
    pub id: usize,

    /// Base strength: worker 1 is the weakest, worker 0 the strongest.
    pub strength: i64,

    /// Soft cost of each task on this worker: worker 2 is the cheapest.
    pub cost: i64,
}
