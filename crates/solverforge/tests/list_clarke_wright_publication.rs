use solverforge::{SolverEvent, SolverManager};

#[path = "list_clarke_wright_publication/domain/mod.rs"]
mod domain;

use domain::{build_plan, PublicationPlan};

#[test]
fn clarke_wright_publishes_constructed_solution_under_solver_manager() {
    static MANAGER: SolverManager<PublicationPlan> = SolverManager::new();

    let plan = build_plan(20, 1);
    let expected_customers = plan.customer_values.len();
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("solve should start");
    let mut saw_non_empty_best = false;

    let completed = loop {
        match receiver
            .blocking_recv()
            .expect("event stream should reach a terminal event")
        {
            SolverEvent::BestSolution { solution, .. } => {
                if solution.routes.iter().any(|route| !route.visits.is_empty()) {
                    saw_non_empty_best = true;
                }
            }
            SolverEvent::Completed { solution, .. } => break solution,
            SolverEvent::Cancelled { metadata } => {
                panic!(
                    "solve was unexpectedly cancelled: {:?}",
                    metadata.terminal_reason
                )
            }
            SolverEvent::Failed { error, .. } => panic!("solve unexpectedly failed: {error}"),
            SolverEvent::Progress { .. }
            | SolverEvent::PauseRequested { .. }
            | SolverEvent::Paused { .. }
            | SolverEvent::Resumed { .. } => {}
        }
    };

    MANAGER
        .delete(job_id)
        .expect("completed job should delete cleanly");

    let assigned_count: usize = completed
        .routes
        .iter()
        .map(|route| route.visits.len())
        .sum();

    assert!(
        saw_non_empty_best,
        "expected a constructed best solution event"
    );
    assert_eq!(assigned_count, expected_customers);
    assert!(
        completed
            .routes
            .iter()
            .any(|route| !route.visits.is_empty()),
        "completed solution should contain constructed routes"
    );
}
