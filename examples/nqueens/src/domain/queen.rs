use solverforge::prelude::*;

#[planning_entity]
pub struct Queen {
    #[planning_id]
    pub id: usize,
    pub column: usize,

    #[planning_variable(value_range_provider = "rows", allows_unassigned = true)]
    pub row_idx: Option<usize>,
}
