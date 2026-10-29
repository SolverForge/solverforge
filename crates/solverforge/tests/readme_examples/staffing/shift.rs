use solverforge::prelude::*;

#[problem_fact]
pub struct Shift {
    #[planning_id]
    pub id: u32,
    pub night: bool,
}
