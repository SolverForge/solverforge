use solverforge::prelude::*;

#[planning_entity]
pub struct Task {
    #[planning_id]
    pub id: usize,

    #[planning_variable(allows_unassigned = "yes")]
    pub worker: Option<usize>,
}

fn main() {}
