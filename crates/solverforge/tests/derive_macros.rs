// Integration tests for derive macros.

use solverforge::__internal::{PlanningId, PlanningSolution as PlanningSolutionTrait};
use solverforge::prelude::*;
use solverforge::stream::CollectionExtract;

#[path = "derive_macros/aliased_route_plan/mod.rs"]
mod aliased_route_plan_domain;
#[path = "derive_macros/duplicate_names/mod.rs"]
mod duplicate_names;
#[path = "derive_macros/route_plan/mod.rs"]
mod route_plan_domain;
#[path = "derive_macros/schedule/mod.rs"]
mod schedule_domain;
#[path = "derive_macros/shadow_plan/mod.rs"]
mod shadow_plan_domain;

use aliased_route_plan_domain::{AliasedRoutePlan, Route as AliasedRoute, Visit as AliasedVisit};
use route_plan_domain::{Route, RoutePlan, Visit};
use schedule_domain::{Employee, Schedule, Shift};
use shadow_plan_domain::{MultiOwnerShadowPlan, RoutedVisit, ShadowRoute, ShadowShift, ShiftVisit};

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
fn test_single_owner_list_sources_are_field_scoped() {
    let plan = RoutePlan {
        visits: vec![Visit { id: 10 }],
        routes: vec![Route {
            id: 1,
            visits: vec![0],
        }],
        score: None,
    };

    let routes = RoutePlan::routes().extract(&plan);
    let visits = RoutePlan::visits().extract(&plan);

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].visits, vec![0]);
    assert_eq!(visits.len(), 1);
    assert_eq!(PlanningId::planning_id(&visits[0]), 10);
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
fn test_multi_owner_list_sources_are_owner_field_scoped() {
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

    let routes = MultiOwnerShadowPlan::routes().extract(&plan);
    let shifts = MultiOwnerShadowPlan::shifts().extract(&plan);
    let routed_visits = MultiOwnerShadowPlan::routed_visits().extract(&plan);
    let shift_visits = MultiOwnerShadowPlan::shift_visits().extract(&plan);

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].visits, vec![0]);
    assert_eq!(shifts.len(), 1);
    assert_eq!(shifts[0].visits, vec![0]);
    assert_eq!(routed_visits.len(), 1);
    assert_eq!(shift_visits.len(), 1);
}

#[test]
fn test_list_sources_work_for_aliased_single_owner_types() {
    let plan = AliasedRoutePlan {
        visits: vec![AliasedVisit { id: 10 }],
        routes: vec![AliasedRoute {
            id: 1,
            visits: vec![0],
        }],
        score: None,
    };

    let routes = AliasedRoutePlan::routes().extract(&plan);
    let visits = AliasedRoutePlan::visits().extract(&plan);

    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].visits, vec![0]);
    assert_eq!(visits.len(), 1);
}

#[test]
fn test_duplicate_short_names_do_not_confuse_list_descriptor_binding() {
    let plan = duplicate_names::Plan {
        visits: vec![duplicate_names::Visit { id: 10 }],
        listed_routes: vec![duplicate_names::Route {
            id: 1,
            visits: vec![0],
        }],
        plain_routes: vec![duplicate_names::RenamedPlainRoute { id: 2 }],
        score: None,
    };

    let listed_routes = duplicate_names::Plan::listed_routes().extract(&plan);
    let plain_routes = duplicate_names::Plan::plain_routes().extract(&plan);
    let descriptor = duplicate_names::Plan::descriptor();
    let listed_descriptor = descriptor
        .find_entity_descriptor("Route")
        .expect("listed route descriptor should be present");
    let plain_descriptor = descriptor
        .find_entity_descriptor("PlainRoute")
        .expect("plain route descriptor should be present");

    assert_eq!(listed_routes.len(), 1);
    assert_eq!(listed_routes[0].visits, vec![0]);
    assert_eq!(plain_routes.len(), 1);
    assert!(listed_descriptor.find_variable("visits").is_some());
    assert!(plain_descriptor.find_variable("visits").is_none());
}
