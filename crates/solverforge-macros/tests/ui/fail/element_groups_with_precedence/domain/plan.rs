use solverforge::prelude::*;

use super::{Route, Stop};

#[planning_solution]
pub struct Plan {
    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[problem_fact_collection]
    pub stops: Vec<Stop>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
