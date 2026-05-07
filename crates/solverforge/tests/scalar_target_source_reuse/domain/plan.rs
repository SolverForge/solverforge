use solverforge::prelude::*;

use super::Shift;

#[planning_solution]
pub struct Plan {
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
