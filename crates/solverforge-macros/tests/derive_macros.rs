//! Integration tests for derive macros.
//!
//! These tests verify that the attribute macros compile and produce
//! correct implementations.

use solverforge::prelude::*;
use std::any::Any;

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

    // PlanningId trait is implemented
    assert_eq!(employee.planning_id(), 1);

    // as_any works
    let any: &dyn Any = employee.as_any();
    assert!(any.is::<Employee>());
}

#[test]
fn test_planning_entity_derives_correctly() {
    let shift = Shift {
        id: 42,
        employee_id: Some(1),
    };

    // PlanningId trait is implemented
    assert_eq!(shift.planning_id(), 42);

    // as_any works
    let any: &dyn Any = shift.as_any();
    assert!(any.is::<Shift>());
}

#[test]
fn test_planning_solution_derives_correctly() {
    let schedule = Schedule {
        employees: vec![Employee {
            id: 1,
            name: "Alice".to_string(),
        }],
        shifts: vec![Shift {
            id: 42,
            employee_id: None,
        }],
        score: Some(HardSoftScore::of(0, 0)),
    };

    // PlanningSolution trait is implemented
    assert_eq!(PlanningSolutionTrait::score(&schedule), Some(HardSoftScore::of(0, 0)));

    let mut schedule2 = schedule.clone();
    PlanningSolutionTrait::set_score(&mut schedule2, Some(HardSoftScore::of(-1, -5)));
    assert_eq!(PlanningSolutionTrait::score(&schedule2), Some(HardSoftScore::of(-1, -5)));
}
