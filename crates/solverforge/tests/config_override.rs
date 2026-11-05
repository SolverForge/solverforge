//! `SolverManager::solve_with_override` runs a retained job under a
//! `SolverConfigOverride`.

use solverforge::config::TerminationConfig;
use solverforge::{
    SolverConfigOverride, SolverEvent, SolverEventReceiver, SolverManager, SolverTelemetry,
};

#[path = "config_override/domain/mod.rs"]
mod domain;

use domain::Plan;

static MANAGER: SolverManager<Plan> = SolverManager::new();

fn completed_telemetry(job_id: usize, mut receiver: SolverEventReceiver<Plan>) -> SolverTelemetry {
    let telemetry = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { metadata, .. } => break metadata.telemetry,
            SolverEvent::Failed { error, .. } => panic!("solve failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("solve was cancelled"),
            _ => {}
        }
    };
    MANAGER.delete(job_id).expect("the completed job deletes");
    telemetry
}

#[test]
fn an_override_replaces_the_job_termination() {
    let (job_id, receiver) = MANAGER.solve(Plan::new()).expect("solve starts");
    let configured = completed_telemetry(job_id, receiver);
    assert!(configured.step_count >= 1000, "{}", configured.step_count);

    let overrides = SolverConfigOverride::with_termination(TerminationConfig {
        step_count_limit: Some(10),
        ..TerminationConfig::default()
    });
    let (job_id, receiver) = MANAGER
        .solve_with_override(Plan::new(), overrides)
        .expect("solve starts");
    let overridden = completed_telemetry(job_id, receiver);
    assert!(overridden.step_count <= 10, "{}", overridden.step_count);

    // The override belongs to its job: the next plain solve is configured.
    let (job_id, receiver) = MANAGER.solve(Plan::new()).expect("solve starts");
    assert_eq!(
        completed_telemetry(job_id, receiver).step_count,
        configured.step_count
    );
}
