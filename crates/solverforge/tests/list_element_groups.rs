//! A pick-up and its drop-off form one element group: construction places
//! them together, and group change, group swap and grouped ruin move them
//! together, keeping the pick-up first.

#[path = "list_element_groups/domain/mod.rs"]
mod domain;

use domain::{Deliveries, Stop, Vehicle};
use solverforge::{SolverEvent, SolverManager, SolverTerminalReason};

fn stop(id: usize, job: Option<usize>, pickup: bool, location: i64) -> Stop {
    Stop {
        id,
        job,
        pickup,
        location,
        vehicle: None,
        index: None,
        previous: None,
    }
}

/// Six jobs whose drop-offs lie far from their pick-ups, then three single
/// stops; nothing is assigned.
fn deliveries(scenario: usize) -> Deliveries {
    let routes = [(5, 60), (70, 12), (33, 90), (8, 41), (95, 20), (50, 52)];
    let mut stops = Vec::new();
    for (job, (from, to)) in routes.into_iter().enumerate() {
        stops.push(stop(2 * job, Some(job), true, from));
        stops.push(stop(2 * job + 1, Some(job), false, to));
    }
    for (offset, location) in [25, 77, 64].into_iter().enumerate() {
        stops.push(stop(12 + offset, None, false, location));
    }
    Deliveries {
        vehicles: (0..3)
            .map(|id| Vehicle {
                id,
                stops: Vec::new(),
            })
            .collect(),
        stops,
        score: None,
        scenario,
    }
}

fn solve(plan: Deliveries) -> Result<Deliveries, String> {
    static MANAGER: SolverManager<Deliveries> = SolverManager::new();
    let (_, mut receiver) = MANAGER.solve(plan).map_err(|error| error.to_string())?;
    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert!(matches!(
                    metadata.terminal_reason,
                    Some(
                        SolverTerminalReason::Completed | SolverTerminalReason::TerminatedByConfig
                    )
                ));
                return Ok(solution);
            }
            SolverEvent::Failed { error, .. } => return Err(error.to_string()),
            _ => {}
        }
    }
    Err("the solve ended without a result".to_string())
}

/// Every stop on exactly one vehicle; each job on one vehicle, pick-up first.
fn assert_jobs_intact(solved: &Deliveries) {
    let mut seen: Vec<usize> = solved
        .vehicles
        .iter()
        .flat_map(|vehicle| vehicle.stops.iter().copied())
        .collect();
    seen.sort_unstable();
    assert_eq!(seen, (0..solved.stops.len()).collect::<Vec<_>>());
    for job in 0..6 {
        let vehicle = solved
            .vehicles
            .iter()
            .find(|vehicle| vehicle.stops.contains(&(2 * job)))
            .expect("every pick-up is assigned");
        let at = |stop| vehicle.stops.iter().position(|&made| made == stop);
        assert!(
            at(2 * job + 1).is_some_and(|dropoff| at(2 * job).unwrap() < dropoff),
            "job {job} split or reversed: {:?}",
            solved.vehicles.iter().map(|v| &v.stops).collect::<Vec<_>>()
        );
    }
    assert_eq!(solved.score.map(|score| score.hard()), Some(0));
}

#[test]
fn default_construction_and_search_keep_every_job_together() {
    let solved = solve(deliveries(0)).expect("the default solve completes");
    assert_jobs_intact(&solved);
}

#[test]
fn regret_construction_and_group_moves_keep_every_job_together() {
    let solved = solve(deliveries(1)).expect("the explicit solve completes");
    assert_jobs_intact(&solved);
}

#[test]
fn round_robin_construction_is_rejected_for_element_groups() {
    let error = solve(deliveries(2)).expect_err("round robin would split jobs");
    assert!(
        error.contains("element groups"),
        "unexpected error: {error}"
    );
}
