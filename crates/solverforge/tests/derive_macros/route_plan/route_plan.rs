use solverforge::prelude::*;

use super::{Route, Visit};

#[planning_solution]
pub struct RoutePlan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
