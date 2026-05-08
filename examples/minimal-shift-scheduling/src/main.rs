use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

mod domain;

use domain::{Nurse, Schedule, Shift};

static MANAGER: SolverManager<Schedule> = SolverManager::new();

fn main() {
    let schedule = Schedule {
        nurses: vec![
            Nurse {
                id: 0,
                name: "Amina".to_string(),
            },
            Nurse {
                id: 1,
                name: "Bruno".to_string(),
            },
            Nurse {
                id: 2,
                name: "Chiara".to_string(),
            },
        ],
        shifts: (0..6)
            .flat_map(|day| {
                (0..2).map(move |slot| Shift {
                    id: day * 2 + slot,
                    day: day as i64,
                    slot,
                    required: true,
                    nurse_idx: None,
                })
            })
            .collect(),
        score: None,
    };

    let (job_id, mut events) = MANAGER.solve(schedule).expect("solver job should start");

    while let Some(event) = events.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert!(matches!(
                    metadata.terminal_reason,
                    Some(
                        SolverTerminalReason::Completed | SolverTerminalReason::TerminatedByConfig
                    )
                ));
                println!("score: {}", solution.score.expect("completed score"));
                for shift in solution.shifts {
                    let nurse = shift
                        .nurse_idx
                        .and_then(|idx| solution.nurses.get(idx))
                        .map(|nurse| nurse.name.as_str())
                        .unwrap_or("unassigned");
                    println!("day {} slot {} -> {nurse}", shift.day, shift.slot);
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
