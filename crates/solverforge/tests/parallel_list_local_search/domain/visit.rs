use solverforge::prelude::*;

#[problem_fact]
pub struct Visit {
    #[planning_id]
    pub id: usize,
    pub demand: i64,
    pub route: Option<usize>,
    pub index: Option<usize>,
    pub previous: Option<usize>,
    pub next: Option<usize>,
}
