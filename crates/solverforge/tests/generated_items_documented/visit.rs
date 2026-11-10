use solverforge::prelude::*;

/// A visit a route can take.
#[problem_fact]
pub struct Visit {
    /// Stable visit id.
    #[planning_id]
    pub id: usize,
}
