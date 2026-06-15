use solverforge::{SolverEvent, SolverManager};

#[path = "list_cvrp_k_opt_time_window/domain/mod.rs"]
mod domain;

use domain::{build_plan, Plan};

#[test]
fn stock_cvrp_list_k_opt_rejects_time_window_breaking_reversal() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let initial_route = vec![1, 3, 2, 4];
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

    assert_eq!(completed.routes[0].visits, initial_route);
}
