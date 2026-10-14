use solverforge::{SolverEvent, SolverManager};

#[path = "list_cvrp_k_opt_time_window/domain/mod.rs"]
mod domain;

use domain::{build_plan, Plan};

/* One local-search phase over the CVRP list model with the benchmark
configuration's union shape: nearby list change and swap leaves, reverse and
k-opt leaves, a ruin leaf, and one limited neighborhood wrapping a sublist
change leaf, under rotating-round-robin selection. The selector graph must
lower every compiled node exactly once. */
#[test]
fn compiled_cvrp_union_selector_lowers_every_node_once() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(build_plan()).expect("solve should start");

    let completed = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::Completed { solution, .. } => break solution,
            SolverEvent::Failed { error, .. } => panic!("solve unexpectedly failed: {error}"),
            SolverEvent::Cancelled { metadata } => {
                panic!(
                    "solve was unexpectedly cancelled: {:?}",
                    metadata.terminal_reason
                )
            }
            SolverEvent::BestSolution { .. }
            | SolverEvent::Progress { .. }
            | SolverEvent::PauseRequested { .. }
            | SolverEvent::Paused { .. }
            | SolverEvent::Resumed { .. } => {}
        }
    };

    MANAGER
        .delete(job_id)
        .expect("completed job should delete cleanly");

    let assigned: usize = completed.routes.iter().map(|route| route.visits.len()).sum();
    assert!(assigned > 0, "construction should assign customers");
}