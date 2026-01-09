//! Integration tests for derive macros.

use solverforge::prelude::*;
use solverforge::__internal::{PlanningId, PlanningSolution as PlanningSolutionTrait};

/// A problem fact representing an employee.
#[problem_fact]
pub struct Employee {
    #[planning_id]
    pub id: i64,
    pub name: String,
}

/// A planning entity representing a shift.
#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: i64,

    #[planning_variable(value_range = "employees", allows_unassigned = true)]
    pub employee_id: Option<i64>,
}

/// A planning solution representing a schedule.
#[planning_solution]
pub struct Schedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

#[test]
fn test_problem_fact_derives_correctly() {
    let employee = Employee {
        id: 1,
        name: "Alice".to_string(),
    };
    assert_eq!(PlanningId::planning_id(&employee), 1);
}

#[test]
fn test_planning_entity_derives_correctly() {
    let shift = Shift {
        id: 42,
        employee_id: Some(1),
    };
    assert_eq!(PlanningId::planning_id(&shift), 42);
}

#[test]
fn test_planning_solution_derives_correctly() {
    let schedule = Schedule {
        employees: vec![Employee { id: 1, name: "Alice".to_string() }],
        shifts: vec![Shift { id: 42, employee_id: None }],
        score: Some(HardSoftScore::of(0, 0)),
    };
    assert_eq!(PlanningSolutionTrait::score(&schedule), Some(HardSoftScore::of(0, 0)));

    let mut schedule2 = schedule.clone();
    PlanningSolutionTrait::set_score(&mut schedule2, Some(HardSoftScore::of(-1, -5)));
    assert_eq!(PlanningSolutionTrait::score(&schedule2), Some(HardSoftScore::of(-1, -5)));
}
