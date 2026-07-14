use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    #[planning_variable(chained = true, value_range_provider = "tasks")]
    pub previous: Option<usize>,
}

fn main() {}
