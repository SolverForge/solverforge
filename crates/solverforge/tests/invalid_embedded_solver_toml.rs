//! A syntactically invalid embedded `solver.toml` fails the job with one
//! readable `Failed` event instead of a panic.

use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge::{SolverEvent, SolverManager};

#[path = "invalid_embedded_solver_toml/domain/mod.rs"]
mod domain;

use domain::{Plan, Resource, Task};

static PANICS: AtomicUsize = AtomicUsize::new(0);

#[test]
fn invalid_embedded_solver_toml_fails_the_job_with_the_configuration_error() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();
    std::panic::set_hook(Box::new(|_| {
        PANICS.fetch_add(1, Ordering::SeqCst);
    }));

    let plan = Plan {
        resources: vec![Resource {
            id: "resource-0".to_string(),
        }],
        tasks: vec![Task {
            id: "task-0".to_string(),
            resource_idx: None,
        }],
        score: None,
    };
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("job should start");
    let mut events = Vec::new();
    while let Some(event) = receiver.blocking_recv() {
        let terminal = matches!(
            event,
            SolverEvent::Completed { .. }
                | SolverEvent::Cancelled { .. }
                | SolverEvent::Failed { .. }
        );
        events.push(event);
        if terminal {
            break;
        }
    }
    MANAGER.delete(job_id).expect("terminal job should delete");
    let _ = std::panic::take_hook();

    let [SolverEvent::Failed { error, .. }] = events.as_slice() else {
        panic!(
            "expected exactly one Failed event, got {} events",
            events.len()
        );
    };
    assert!(
        error.starts_with("solver configuration invalid: TOML parse error"),
        "{error}"
    );
    assert!(error.contains("unclosed array table"), "{error}");
    assert!(!error.contains("Toml(Error {"), "{error}");
    assert_eq!(PANICS.load(Ordering::SeqCst), 0, "no thread may panic");
}
