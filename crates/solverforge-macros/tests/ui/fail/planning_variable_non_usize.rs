use solverforge::prelude::*;

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: usize,

    #[planning_variable(value_range_provider = "employees", allows_unassigned = true)]
    pub employee_idx: Option<u32>,
}

fn main() {}
