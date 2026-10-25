use solverforge::{
    CandidateTraceDisposition, SolverEvent, SolverManager, SolverTelemetry, SolverTerminalReason,
};

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

struct Run {
    plan: Plan,
    telemetry: SolverTelemetry,
    consumed_pulls: Vec<String>,
}

fn solve(manager: &'static SolverManager<Plan>, move_threads: usize) -> Run {
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
    let (plan, telemetry) = completed.expect("expected a completed solution");
    let trace = manager
        .get_telemetry_detail(job_id)
        .expect("completed job keeps its telemetry detail")
        .candidate_trace
        .expect("the fixture enables the candidate trace");
    assert!(
        !trace.truncated,
        "the trace cap must cover the whole solve: {} pulls",
        trace.total_pulls
    );
    // Speculative pulls are the only legitimate difference between thread
    // counts; every consumed candidate and its decisions must match in order.
    let consumed_pulls = trace
        .pulls
        .iter()
        .filter(|pull| {
            !pull
                .dispositions
                .contains(&CandidateTraceDisposition::DiscardedSpeculative)
        })
        .map(|pull| {
            format!(
                "{}:{}:{:?}:{:?}",
                pull.phase_index, pull.step_index, pull.identity, pull.dispositions
            )
        })
        .collect();
    manager.delete(job_id).expect("delete completed job");
    Run {
        plan,
        telemetry,
        consumed_pulls,
    }
}

#[test]
fn fixed_work_trajectory_is_identical_across_thread_counts() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let runs: Vec<_> = [1, 2, 4]
        .into_iter()
        .map(|move_threads| solve(&MANAGER, move_threads))
        .collect();
    let Run {
        plan: baseline,
        telemetry: baseline_telemetry,
        consumed_pulls: baseline_pulls,
    } = &runs[0];
    let baseline_assignment: Vec<_> = baseline
        .tasks
        .iter()
        .map(|task| task.resource_idx)
        .collect();
    assert!(baseline_telemetry.moves_applied > 0);
    assert!(!baseline_pulls.is_empty());

    for Run {
        plan: solution,
        telemetry,
        consumed_pulls,
    } in &runs[1..]
    {
        assert_eq!(consumed_pulls, baseline_pulls);
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
        let run = solve(&MANAGER, move_threads);
        assert_eq!(run.telemetry.moves_speculative, 0);
    }
}
