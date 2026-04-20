// Integration tests for derive macros.

use solverforge::__internal::{PlanningId, PlanningSolution as PlanningSolutionTrait};
use solverforge::prelude::*;

// A problem fact representing an employee.
#[problem_fact]
pub struct Employee {
    #[planning_id]
    pub id: i64,
    pub name: String,
}

#[problem_fact]
pub struct Visit {
    #[planning_id]
    pub id: i64,
}

// A planning entity representing a shift.
#[planning_entity]
pub struct Shift {
    #[planning_id]
    pub id: i64,

    #[planning_variable(value_range = "employees", allows_unassigned = true)]
    pub employee_id: Option<i64>,
}

#[planning_entity]
pub struct Route {
    #[planning_id]
    pub id: i64,

    #[planning_list_variable(element_collection = "visits")]
    pub visits: Vec<usize>,
}

// A planning solution representing a schedule.
#[planning_solution]
pub struct Schedule {
    #[problem_fact_collection]
    pub employees: Vec<Employee>,

    #[planning_entity_collection]
    pub shifts: Vec<Shift>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

#[planning_solution]
pub struct RoutePlan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<Route>,

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
    assert_eq!(
        Employee::problem_fact_descriptor("employees").id_field,
        Some("id")
    );
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
    assert_eq!(
        PlanningSolutionTrait::score(&schedule),
        Some(HardSoftScore::of(0, 0))
    );

    let mut schedule2 = schedule.clone();
    PlanningSolutionTrait::set_score(&mut schedule2, Some(HardSoftScore::of(-1, -5)));
    assert_eq!(
        PlanningSolutionTrait::score(&schedule2),
        Some(HardSoftScore::of(-1, -5))
    );
}

#[test]
fn test_solution_descriptor_preserves_entity_variable_metadata() {
    let descriptor = Schedule::descriptor();
    let shift_descriptor = descriptor
        .find_entity_descriptor("Shift")
        .expect("Shift descriptor should be present");

    assert_eq!(shift_descriptor.solution_field, "shifts");
    assert_eq!(shift_descriptor.id_field, Some("id"));

    let employee_var = shift_descriptor
        .find_variable("employee_id")
        .expect("employee_id variable descriptor should be present");

    assert!(employee_var.allows_unassigned);
    assert_eq!(employee_var.value_range_provider, Some("employees"));
}

#[test]
fn test_field_only_list_solution_preserves_list_descriptor_metadata() {
    let descriptor = RoutePlan::descriptor();
    let route_descriptor = descriptor
        .find_entity_descriptor("Route")
        .expect("Route descriptor should be present");

    assert_eq!(route_descriptor.solution_field, "routes");
    assert_eq!(route_descriptor.id_field, Some("id"));

    let visits_var = route_descriptor
        .find_variable("visits")
        .expect("visits variable descriptor should be present");

    assert_eq!(visits_var.name, "visits");
    assert_eq!(
        visits_var.variable_type,
        solverforge_core::domain::VariableType::List
    );
}
