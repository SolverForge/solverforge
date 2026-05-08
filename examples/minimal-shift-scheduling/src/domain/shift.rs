use solverforge::prelude::*;

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: usize,
    pub day: i64,
    pub slot: usize,
    pub required: bool,

    #[planning_variable(value_range_provider = "nurses", allows_unassigned = true)]
    pub nurse_idx: Option<usize>,
}
