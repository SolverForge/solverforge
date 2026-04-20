use solverforge::prelude::*;

use crate::{Container, Item};

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub items: Vec<Item>,

    #[planning_entity_collection]
    pub containers: Vec<Container>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
