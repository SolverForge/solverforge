use solverforge::prelude::*;

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: i64,

    #[planning_variable(value_range = "employees", allows_unassigned = true)]
    pub employee_id: Option<i64>,
}
