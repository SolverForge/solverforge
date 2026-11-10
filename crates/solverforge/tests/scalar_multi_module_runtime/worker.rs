use solverforge::prelude::*;

/// A worker tasks can be assigned to.
#[problem_fact]
pub struct Worker {
    /// Stable worker id.
    #[planning_id]
    pub id: usize,
}
