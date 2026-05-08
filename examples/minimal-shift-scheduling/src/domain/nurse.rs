use solverforge::prelude::*;

#[problem_fact]
pub struct Nurse {
    #[planning_id]
    pub id: usize,
    pub name: String,
}
