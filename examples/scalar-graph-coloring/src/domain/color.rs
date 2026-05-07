use solverforge::prelude::*;

#[problem_fact]
pub struct Color {
    #[planning_id]
    pub id: usize,
    pub name: String,
}
