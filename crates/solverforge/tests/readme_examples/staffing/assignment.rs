use solverforge::prelude::*;

#[planning_entity]
pub struct Assignment {
    #[planning_id]
    pub id: usize,
    pub shift_id: u32,
    pub employee_code: String,
}
