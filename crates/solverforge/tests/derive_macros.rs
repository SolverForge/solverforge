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

#[problem_fact]
pub struct RoutedVisit {
    #[planning_id]
    pub id: usize,
    pub route: Option<usize>,
}

#[problem_fact]
pub struct ShiftVisit {
    #[planning_id]
    pub id: usize,
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

#[planning_entity]
pub struct ShadowRoute {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "routed_visits")]
    pub visits: Vec<usize>,
}

#[planning_entity]
pub struct ShadowShift {
    #[planning_id]
    pub id: usize,

    #[planning_list_variable(element_collection = "shift_visits")]
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

type VehicleRoute = Route;

#[planning_solution]
pub struct AliasedRoutePlan {
    #[problem_fact_collection]
    pub visits: Vec<Visit>,

    #[planning_entity_collection]
    pub routes: Vec<VehicleRoute>,

    #[planning_score]
    pub score: Option<HardSoftScore>,
}

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

mod duplicate_names {
    use solverforge::prelude::*;

    use super::Visit;

    #[planning_entity]
    pub struct Route {
        #[planning_id]
        pub id: usize,

        #[planning_list_variable(element_collection = "visits")]
        pub visits: Vec<usize>,
    }

    #[planning_entity]
    pub struct PlainRoute {
        #[planning_id]
        pub id: usize,
    }

    pub type RenamedPlainRoute = PlainRoute;

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

#[test]
fn test_single_owner_list_helpers_remain_available_when_unambiguous() {
    let mut plan = RoutePlan {
        visits: vec![Visit { id: 10 }],
        routes: vec![Route {
            id: 1,
            visits: vec![0],
        }],
        score: None,
    };

    assert_eq!(plan.list_len(0), 1);
    assert_eq!(RoutePlan::list_len_static(&plan, 0), 1);
    assert_eq!(RoutePlan::element_count(&plan), 1);
    assert_eq!(RoutePlan::n_entities(&plan), 1);
    assert_eq!(RoutePlan::assigned_elements(&plan), vec![0]);
    assert_eq!(RoutePlan::index_to_element_static(&plan, 0), 0);
    assert_eq!(RoutePlan::list_variable_descriptor_index(), 0);

    assert_eq!(RoutePlan::routes_list_len_static(&plan, 0), 1);
    assert_eq!(RoutePlan::routes_element_count(&plan), 1);
    assert_eq!(RoutePlan::routes_n_entities(&plan), 1);
    assert_eq!(RoutePlan::routes_assigned_elements(&plan), vec![0]);
    assert_eq!(RoutePlan::routes_index_to_element_static(&plan, 0), 0);
    assert_eq!(RoutePlan::routes_list_variable_descriptor_index(), 0);

    RoutePlan::assign_element(&mut plan, 0, 0);
    RoutePlan::routes_assign_element(&mut plan, 0, 0);
    assert_eq!(RoutePlan::list_len_static(&plan, 0), 3);
}

#[test]
fn test_multi_owner_shadow_updates_are_descriptor_scoped() {
    let mut plan = MultiOwnerShadowPlan {
        routes: vec![ShadowRoute {
            id: 1,
            visits: vec![0],
        }],
        shifts: vec![ShadowShift {
            id: 2,
            visits: vec![0],
        }],
        routed_visits: vec![RoutedVisit {
            id: 10,
            route: None,
        }],
        shift_visits: vec![ShiftVisit { id: 20 }],
        score: None,
    };

    <MultiOwnerShadowPlan as PlanningSolutionTrait>::update_entity_shadows(&mut plan, 1, 0);
    assert_eq!(plan.routed_visits[0].route, None);

    <MultiOwnerShadowPlan as PlanningSolutionTrait>::update_entity_shadows(&mut plan, 0, 0);
    assert_eq!(plan.routed_visits[0].route, Some(0));

    plan.routed_visits[0].route = None;
    <MultiOwnerShadowPlan as PlanningSolutionTrait>::update_all_shadows(&mut plan);
    assert_eq!(plan.routed_visits[0].route, Some(0));
}

#[test]
fn test_multi_owner_list_helpers_are_owner_scoped() {
    let mut plan = MultiOwnerShadowPlan {
        routes: vec![ShadowRoute {
            id: 1,
            visits: vec![0],
        }],
        shifts: vec![ShadowShift {
            id: 2,
            visits: vec![0],
        }],
        routed_visits: vec![RoutedVisit {
            id: 10,
            route: None,
        }],
        shift_visits: vec![ShiftVisit { id: 20 }],
        score: None,
    };

    assert_eq!(MultiOwnerShadowPlan::routes_list_len_static(&plan, 0), 1);
    assert_eq!(MultiOwnerShadowPlan::shifts_list_len_static(&plan, 0), 1);
    assert_eq!(MultiOwnerShadowPlan::routes_element_count(&plan), 1);
    assert_eq!(MultiOwnerShadowPlan::shifts_element_count(&plan), 1);
    assert_eq!(MultiOwnerShadowPlan::routes_n_entities(&plan), 1);
    assert_eq!(MultiOwnerShadowPlan::shifts_n_entities(&plan), 1);
    assert_eq!(
        MultiOwnerShadowPlan::routes_assigned_elements(&plan),
        vec![0]
    );
    assert_eq!(
        MultiOwnerShadowPlan::shifts_assigned_elements(&plan),
        vec![0]
    );
    assert_eq!(
        MultiOwnerShadowPlan::routes_index_to_element_static(&plan, 0),
        0
    );
    assert_eq!(
        MultiOwnerShadowPlan::shifts_index_to_element_static(&plan, 0),
        0
    );
    assert_eq!(
        MultiOwnerShadowPlan::routes_list_variable_descriptor_index(),
        0
    );
    assert_eq!(
        MultiOwnerShadowPlan::shifts_list_variable_descriptor_index(),
        1
    );

    MultiOwnerShadowPlan::routes_assign_element(&mut plan, 0, 0);
    MultiOwnerShadowPlan::shifts_assign_element(&mut plan, 0, 0);
    assert_eq!(MultiOwnerShadowPlan::routes_list_len_static(&plan, 0), 2);
    assert_eq!(MultiOwnerShadowPlan::shifts_list_len_static(&plan, 0), 2);
}

#[test]
fn test_list_helpers_work_for_aliased_single_owner_types() {
    let mut plan = AliasedRoutePlan {
        visits: vec![Visit { id: 10 }],
        routes: vec![Route {
            id: 1,
            visits: vec![0],
        }],
        score: None,
    };

    assert_eq!(AliasedRoutePlan::list_len_static(&plan, 0), 1);
    assert_eq!(AliasedRoutePlan::routes_list_len_static(&plan, 0), 1);
    assert_eq!(AliasedRoutePlan::routes_element_count(&plan), 1);

    AliasedRoutePlan::assign_element(&mut plan, 0, 0);
    AliasedRoutePlan::routes_assign_element(&mut plan, 0, 0);
    assert_eq!(AliasedRoutePlan::list_len_static(&plan, 0), 3);
}

#[test]
fn test_duplicate_short_names_do_not_confuse_list_helper_binding() {
    let plan = duplicate_names::Plan {
        visits: vec![Visit { id: 10 }],
        listed_routes: vec![duplicate_names::Route {
            id: 1,
            visits: vec![0],
        }],
        plain_routes: vec![duplicate_names::RenamedPlainRoute { id: 2 }],
        score: None,
    };

    assert_eq!(duplicate_names::Plan::list_len_static(&plan, 0), 1);
    assert_eq!(
        duplicate_names::Plan::listed_routes_list_len_static(&plan, 0),
        1
    );

    let panic = std::panic::catch_unwind(|| {
        let _ = duplicate_names::Plan::plain_routes_list_len_static(&plan, 0);
    })
    .expect_err("non-list entity collections should reject list helper calls");

    let message = if let Some(message) = panic.downcast_ref::<String>() {
        message.as_str()
    } else if let Some(message) = panic.downcast_ref::<&str>() {
        message
    } else {
        ""
    };
    assert!(message.contains("plain_routes"));
}

#[test]
fn test_multi_owner_generic_list_helpers_reject_ambiguous_calls() {
    let plan = MultiOwnerShadowPlan {
        routes: vec![ShadowRoute {
            id: 1,
            visits: vec![0],
        }],
        shifts: vec![ShadowShift {
            id: 2,
            visits: vec![0],
        }],
        routed_visits: vec![RoutedVisit {
            id: 10,
            route: None,
        }],
        shift_visits: vec![ShiftVisit { id: 20 }],
        score: None,
    };

    let panic = std::panic::catch_unwind(|| {
        let _ = MultiOwnerShadowPlan::list_len_static(&plan, 0);
    })
    .expect_err("multi-owner plans should reject generic list helper calls");

    let message = if let Some(message) = panic.downcast_ref::<String>() {
        message.as_str()
    } else if let Some(message) = panic.downcast_ref::<&str>() {
        message
    } else {
        ""
    };
    assert!(message.contains("single-owner list helper"));
}
