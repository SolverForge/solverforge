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
                    pinned: false,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_schedule() -> Schedule {
        Schedule {
            nurses: (0..3)
                .map(|id| Nurse {
                    id,
                    name: id.to_string(),
                })
                .collect(),
            shifts: (0..6)
                .flat_map(|day| {
                    (0..2).map(move |slot| Shift {
                        id: day * 2 + slot,
                        day: day as i64,
                        slot,
                        required: true,
                        pinned: false,
                        nurse_idx: None,
                    })
                })
                .collect(),
            score: None,
        }
    }

    #[test]
    fn pinned_shifts_keep_their_input_assignment_through_configured_phases() {
        let mut schedule = test_schedule();
        schedule.shifts[0].pinned = true;
        schedule.shifts[0].nurse_idx = Some(0);
        let (job_id, mut events) = MANAGER.solve(schedule).expect("solve should start");
        while let Some(event) = events.blocking_recv() {
            match event {
                SolverEvent::Completed { solution, .. } => {
                    assert_eq!(solution.shifts[0].nurse_idx, Some(0));
                    assert!(solution
                        .shifts
                        .iter()
                        .all(|shift| shift.nurse_idx.is_some()));
                    MANAGER.delete(job_id).expect("delete completed job");
                    return;
                }
                SolverEvent::Failed { error, .. } => panic!("solver failed: {error}"),
                _ => {}
            }
        }
        panic!("solver ended without completion");
    }

    #[test]
    fn fully_pinned_roster_is_not_reassigned_by_local_search() {
        let mut schedule = test_schedule();
        for shift in &mut schedule.shifts {
            shift.pinned = true;
            shift.nurse_idx = Some(0);
        }
        let (job_id, mut events) = MANAGER.solve(schedule).expect("solve should start");
        while let Some(event) = events.blocking_recv() {
            match event {
                SolverEvent::Completed { solution, .. } => {
                    assert_eq!(solution.shifts.len(), 12);
                    assert!(solution
                        .shifts
                        .iter()
                        .all(|shift| shift.nurse_idx == Some(0)));
                    MANAGER.delete(job_id).expect("delete completed job");
                    return;
                }
                SolverEvent::Failed { error, .. } => panic!("solver failed: {error}"),
                _ => {}
            }
        }
        panic!("solver ended without completion");
    }

    #[test]
    fn pinned_required_shift_without_an_assignment_fails_completion() {
        let schedule = Schedule {
            nurses: vec![Nurse {
                id: 0,
                name: "Amina".into(),
            }],
            shifts: vec![Shift {
                id: 0,
                day: 0,
                slot: 0,
                required: true,
                pinned: true,
                nurse_idx: None,
            }],
            score: None,
        };
        let (job_id, mut events) = MANAGER.solve(schedule).expect("solve should start");
        while let Some(event) = events.blocking_recv() {
            match event {
                SolverEvent::Failed { error, .. } => {
                    assert!(
                        error.contains("mandatory planning work incomplete"),
                        "{error}"
                    );
                    MANAGER.delete(job_id).expect("delete failed job");
                    return;
                }
                SolverEvent::Completed { .. } => panic!("incomplete pinned shift was published"),
                _ => {}
            }
        }
        panic!("solver ended without reporting incomplete mandatory work");
    }

    #[test]
    fn optional_pinned_shift_may_remain_unassigned() {
        let schedule = Schedule {
            nurses: vec![Nurse {
                id: 0,
                name: "Amina".into(),
            }],
            shifts: vec![Shift {
                id: 0,
                day: 0,
                slot: 0,
                required: false,
                pinned: true,
                nurse_idx: None,
            }],
            score: None,
        };
        let (job_id, mut events) = MANAGER.solve(schedule).expect("solve should start");
        while let Some(event) = events.blocking_recv() {
            match event {
                SolverEvent::Completed { solution, .. } => {
                    assert_eq!(solution.shifts[0].nurse_idx, None);
                    MANAGER.delete(job_id).expect("delete completed job");
                    return;
                }
                SolverEvent::Failed { error, .. } => panic!("solver failed: {error}"),
                _ => {}
            }
        }
        panic!("solver ended without completion");
    }
}
