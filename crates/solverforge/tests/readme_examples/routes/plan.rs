use solverforge::prelude::*;

use super::Visit;

// readme:begin
#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "visits")]
    pub visit_order: Vec<usize>,
}

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
// readme:end
