use solverforge::prelude::*;

#[planning_entity]
pub struct Operation {
    #[planning_id]
    pub id: usize,
    pub job: usize,
    pub step: usize,

    #[planning_variable(value_range_provider = "machines", allows_unassigned = true)]
    pub machine_idx: Option<usize>,
}
