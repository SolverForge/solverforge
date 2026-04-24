use solverforge::prelude::*;

#[planning_entity]
pub struct PlainRoute {
    #[planning_id]
    pub id: usize,
}

pub type RenamedPlainRoute = PlainRoute;
