use solverforge::prelude::*;

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: usize,
    pub minutes: i64,
}
