use solverforge::{SolverEvent, SolverManager, SolverTelemetry, SolverTerminalReason};

#[path = "parallel_local_search/domain/mod.rs"]
mod domain;

use domain::{Plan, Resource, Task};

fn seeded_plan(move_threads: usize) -> Plan {
    let resources = (0..6)
        .map(|idx| Resource {
            id: format!("resource-{idx}"),
        })
        .collect();
    let tasks = (0..40)
        .map(|idx| Task {
            id: format!("task-{idx}"),
            weight: (idx * 7 + 3) % 11,
            resource_idx: None,
        })
        .collect();

    Plan {
        resources,
        tasks,
        score: None,
        move_threads,
    }
}

fn solve(manager: &'static SolverManager<Plan>, move_threads: usize) -> (Plan, SolverTelemetry) {
    let (job_id, mut receiver) = manager
        .solve(seeded_plan(move_threads))
        .expect("job should start");
    let mut completed = None;
    while let Some(event) = receiver.blocking_recv() {
        if let SolverEvent::Completed { metadata, solution } = event {
            assert_eq!(
                metadata.terminal_reason,
                Some(SolverTerminalReason::Completed)
            );
            completed = Some((solution, metadata.telemetry));
            break;
        }
    }
    manager.delete(job_id).expect("delete completed job");
    completed.expect("expected a completed solution")
}

#[test]
fn fixed_work_trajectory_is_identical_across_thread_counts() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let runs: Vec<_> = [1, 2, 4]
        .into_iter()
        .map(|move_threads| solve(&MANAGER, move_threads))
        .collect();
    let (baseline, baseline_telemetry) = &runs[0];
    let baseline_assignment: Vec<_> = baseline
        .tasks
        .iter()
        .map(|task| task.resource_idx)
        .collect();
    assert!(baseline_telemetry.moves_applied > 0);

    for (solution, telemetry) in &runs[1..] {
        let assignment: Vec<_> = solution
            .tasks
            .iter()
            .map(|task| task.resource_idx)
            .collect();
        assert_eq!(assignment, baseline_assignment);
        assert_eq!(solution.score, baseline.score);
        assert_eq!(telemetry.step_count, baseline_telemetry.step_count);
        assert_eq!(
            telemetry.moves_evaluated,
            baseline_telemetry.moves_evaluated
        );
        assert_eq!(telemetry.moves_applied, baseline_telemetry.moves_applied);
    }
}

#[test]
fn sequential_resolution_reports_no_speculation() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    for move_threads in [0, 1] {
        let (_, telemetry) = solve(&MANAGER, move_threads);
        assert_eq!(telemetry.moves_speculative, 0);
    }
}
