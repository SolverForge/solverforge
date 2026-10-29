use solverforge::prelude::*;

use super::{Assignment, Employee, Shift};

#[planning_solution]
pub struct Schedule {
    #[problem_fact_collection]
    pub shifts: Vec<Shift>,

    #[problem_fact_collection]
    pub employees: Vec<Employee>,

    #[planning_entity_collection]
    pub assignments: Vec<Assignment>,

    #[planning_score]
    pub score: Option<SoftScore>,
}
