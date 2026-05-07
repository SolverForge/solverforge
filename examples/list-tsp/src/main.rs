use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

mod domain;

use domain::{Route, TourPlan, Visit};

static MANAGER: SolverManager<TourPlan> = SolverManager::new();

fn main() {
    let visit_values: Vec<usize> = (1..=4).collect();
    let plan = TourPlan {
        visits: visit_values
            .iter()
            .copied()
            .map(|id| Visit { id })
            .collect(),
        visit_values,
        routes: vec![Route {
            id: 0,
            visits: Vec::new(),
        }],
        score: None,
    };

    let (job_id, mut events) = MANAGER.solve(plan).expect("solver job should start");

    while let Some(event) = events.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                println!("score: {}", solution.score.expect("completed score"));
                for route in solution.routes {
                    println!("route {} -> {:?}", route.id, route.visits);
                }
                MANAGER.delete(job_id).expect("delete completed job");
                break;
            }
            SolverEvent::Failed { error, .. } => panic!("solver failed: {error}"),
            SolverEvent::Cancelled { .. } => panic!("solver was cancelled"),
            _ => {}
        }
    }
}
