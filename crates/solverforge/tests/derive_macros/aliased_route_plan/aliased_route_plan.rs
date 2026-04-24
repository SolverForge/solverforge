use solverforge::prelude::*;

use super::{VehicleRoute, Visit};

#[planning_solution]
pub struct AliasedRoutePlan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<VehicleRoute>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
