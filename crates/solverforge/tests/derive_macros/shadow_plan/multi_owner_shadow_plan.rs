use solverforge::prelude::*;

use super::{RoutedVisit, ShadowRoute, ShadowShift, ShiftVisit};

#[planning_solution]
#[shadow_variable_updates(list_owner = "routes", inverse_field = "route")]
pub struct MultiOwnerShadowPlan {
    #[planning_entity_collection]
    pub routes: Vec<ShadowRoute>,

    #[planning_entity_collection]
    pub shifts: Vec<ShadowShift>,

    #[problem_fact_collection]
    pub routed_visits: Vec<RoutedVisit>,

    #[problem_fact_collection]
    pub shift_visits: Vec<ShiftVisit>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
