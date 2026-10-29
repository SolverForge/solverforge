use solverforge::prelude::*;

use super::{Employee, Shift, Visit};

#[planning_solution]
pub struct Plan {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[planning_entity_collection]
    pub visits: Vec<Visit>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}
