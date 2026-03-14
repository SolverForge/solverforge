use serde::{Deserialize, Serialize};
use solverforge::prelude::*;

use super::{Employee, Shift};

/// The root planning solution: employees + shifts + score.
#[planning_solution(constraints = "crate::constraints::create_constraints")]
#[basic_variable_config(
    entity_collection = "shifts",
    variable_field = "employee_idx",
    variable_type = "usize",
    value_range = "employees"
)]
#[derive(Serialize, Deserialize)]
pub struct EmployeeSchedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,
    #[planning_entity_collection]
    pub shifts: Vec<Shift>,
    #[planning_score]
    pub score: Option<HardSoftDecimalScore>,
    #[serde(rename = "solverStatus", skip_serializing_if = "Option::is_none")]
    pub solver_status: Option<String>,
}

impl EmployeeSchedule {
    pub fn new(employees: Vec<Employee>, shifts: Vec<Shift>) -> Self {
        Self {
            employees,
            shifts,
            score: None,
            solver_status: None,
        }
    }

    #[inline]
    pub fn get_employee(&self, idx: usize) -> Option<&Employee> {
        self.employees.get(idx)
    }
}
