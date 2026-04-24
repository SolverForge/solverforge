use solverforge::{SolverEvent, SolverLifecycleState, SolverManager};

#[path = "scalar_runtime_selector_assembly/domain/mod.rs"]
mod domain;

use domain::{Plan, Resource, Task};

fn seeded_plan() -> Plan {
    let resources = (0..4)
        .map(|idx| Resource {
            id: format!("resource-{idx}"),
        })
        .collect();
    let tasks = (0..12)
        .map(|idx| Task {
            id: format!("task-{idx}"),
            resource_idx: None,
        })
        .collect();

    Plan {
        resources,
        tasks,
        score: None,
    }
}

#[test]
fn scalar_only_solution_builds_default_selector_without_failure() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan()).expect("job should start");
    let mut completed = false;

    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::BestSolution { .. } => {}
            SolverEvent::Completed { metadata, .. } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
                completed = true;
                break;
            }
            SolverEvent::Failed { error, .. } => {
                panic!("scalar runtime selector assembly failed: {error}");
            }
            _ => {}
        }
    }

    assert!(completed, "expected selector assembly solve to complete");
    MANAGER.delete(job_id).expect("delete completed job");
}
