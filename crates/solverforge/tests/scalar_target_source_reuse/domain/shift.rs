use solverforge::prelude::*;

#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: i64,

    #[planning_variable]
    pub primary: Option<i64>,

    #[planning_variable]
    pub secondary: Option<i64>,
}
