#[path = "scalar_multi_module_runtime/mod.rs"]
mod domain;

pub use domain::{Plan, Task, WorkTask, Worker};

use solverforge::{SolverEvent, SolverLifecycleState, SolverManager};

fn seeded_plan() -> Plan {
    Plan {
        workers: (0..3).map(|id| Worker { id }).collect(),
        tasks: (0..8).map(|id| Task { id, worker: None }).collect(),
        score: None,
    }
}

#[test]
fn planning_model_attaches_scalar_hooks_from_entity_module() {
    let descriptor = Plan::descriptor();
    let task_descriptor = descriptor
        .find_entity_descriptor("Task")
        .expect("task descriptor should exist");
    let worker_variable = task_descriptor
        .find_variable("worker")
        .expect("worker variable should exist");

    assert!(worker_variable.nearby_value_distance_meter.is_some());
    assert!(worker_variable.nearby_entity_distance_meter.is_some());
    assert!(worker_variable.construction_entity_order_key.is_some());
    assert!(worker_variable.construction_value_order_key.is_some());
}

#[test]
fn scalar_runtime_survives_solution_module_before_entity_module() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan()).expect("job should start");
    let mut completed = None;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
                completed = Some((metadata, solution));
                break;
            }
            SolverEvent::Failed { error, .. } => {
                panic!("multi-module scalar runtime solve failed: {error}");
            }
            _ => {}
        }
    }

    let (metadata, solution) = completed.expect("expected completed solve");
    assert!(
        solution.tasks.iter().all(|task| task.worker.is_some()),
        "scalar construction should assign every task"
    );
    assert!(
        metadata.telemetry.moves_generated > 0,
        "local search should generate scalar candidates"
    );
    assert!(
        metadata.telemetry.moves_evaluated > 0,
        "local search should evaluate scalar candidates"
    );

    MANAGER.delete(job_id).expect("delete completed job");
}
