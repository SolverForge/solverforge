use solverforge::prelude::*;

use super::{Operation, Route};

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub operations: Vec<Operation>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
