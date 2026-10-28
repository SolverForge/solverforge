use solverforge::prelude::*;

#[problem_fact]
pub struct Worker {
    #[planning_id]
    pub id: usize,
    pub cost: i64,
}
