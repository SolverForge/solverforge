use solverforge::prelude::*;

#[planning_entity]
pub struct Visit {
    #[planning_id]
    pub id: usize,
    pub hours: i64,

    #[planning_variable(value_range_provider = "employees", allows_unassigned = true)]
    pub employee_idx: Option<usize>,
}
