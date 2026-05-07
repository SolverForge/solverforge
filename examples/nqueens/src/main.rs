use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

mod domain;

use domain::{Board, Queen, Row};

static MANAGER: SolverManager<Board> = SolverManager::new();

fn main() {
    let seed_rows = [0, 4, 7, 5, 2, 6, 1, 3];
    let board = Board {
        rows: (0..8).map(|id| Row { id }).collect(),
        queens: (0..8)
            .map(|column| Queen {
                id: column,
                column,
                row_idx: Some(seed_rows[column]),
            })
            .collect(),
        score: None,
    };

    let (job_id, mut events) = MANAGER.solve(board).expect("solver job should start");

    while let Some(event) = events.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(
                    metadata.terminal_reason,
                    Some(SolverTerminalReason::Completed)
                );
                println!("score: {}", solution.score.expect("completed score"));
                for queen in solution.queens {
                    println!("column {} -> row {:?}", queen.column, queen.row_idx);
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
