#[path = "mixed_variable_order_runtime/mod.rs"]
mod domain;

pub use domain::{Plan, Route, Visit};

use solverforge::{SolverEvent, SolverLifecycleState, SolverManager};

fn seeded_plan() -> Plan {
    Plan {
        routes: vec![Route {
            id: 0,
            visits: Vec::new(),
            first_visit: None,
        }],
        visits: vec![Visit { id: 0 }],
        score: None,
    }
}

#[test]
fn generic_construction_orders_list_variable_before_scalar_variable() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let descriptor = Plan::descriptor();
    let route_descriptor = descriptor
        .find_entity_descriptor("Route")
        .expect("route descriptor should exist");
    let visits_order = route_descriptor
        .variable_descriptors
        .iter()
        .position(|variable| variable.name == "visits")
        .expect("list variable descriptor should exist");
    let first_visit_order = route_descriptor
        .variable_descriptors
        .iter()
        .position(|variable| variable.name == "first_visit")
        .expect("scalar variable descriptor should exist");
    assert!(visits_order < first_visit_order);

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan()).expect("job should start");
    let mut completed = None;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
                completed = Some(solution);
                break;
            }
            SolverEvent::Failed { error, .. } => {
                panic!("mixed construction solve failed: {error}");
            }
            _ => {}
        }
    }

    let solution = completed.expect("expected completed solve");
    assert_eq!(solution.routes[0].visits, vec![0]);
    assert_eq!(
        solution.routes[0].first_visit,
        Some(0),
        "scalar entity-field value range should see the list mutation before it is constructed"
    );

    MANAGER.delete(job_id).expect("delete completed job");
}
