use solverforge::prelude::*;

use super::{RenamedPlainRoute, Route, Visit};

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub listed_routes: Vec<Route>,

    #[planning_entity_collection]
    pub plain_routes: Vec<RenamedPlainRoute>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
