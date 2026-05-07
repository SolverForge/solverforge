use solverforge::prelude::*;

#[planning_entity]
pub struct Node {
    #[planning_id]
    pub id: usize,
    pub neighbors: Vec<usize>,

    #[planning_variable(value_range_provider = "colors", allows_unassigned = true)]
    pub color_idx: Option<usize>,
}
