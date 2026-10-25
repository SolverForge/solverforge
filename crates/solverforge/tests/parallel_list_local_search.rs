use solverforge::{
    AppliedMoveTelemetry, CandidateTraceDisposition, SolverEvent, SolverManager, SolverTelemetry,
    SolverTerminalReason,
};

#[path = "parallel_list_local_search/domain/mod.rs"]
mod domain;

use domain::{Route, RoutePlan, Visit};

fn seeded_plan(move_threads: usize) -> RoutePlan {
    let routes = (0..3)
        .map(|id| Route {
            id,
            visits: Vec::new(),
        })
        .collect();
    let visits = (0..18)
        .map(|id| Visit {
            id,
            demand: (id as i64 * 5 + 2) % 9,
            route: None,
            index: None,
            previous: None,
            next: None,
        })
        .collect();
    RoutePlan {
        routes,
        visits,
        score: None,
        move_threads,
    }
}

struct Run {
    plan: RoutePlan,
    telemetry: SolverTelemetry,
    consumed_pulls: Vec<String>,
}

fn solve(manager: &'static SolverManager<RoutePlan>, move_threads: usize) -> Run {
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
    // counts: every candidate the coordinator consumed, and what it decided
    // about it, must match the sequential run in order.
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

fn selected_moves(trace: &[AppliedMoveTelemetry]) -> Vec<(u64, &'static str, usize, f64)> {
    trace
        .iter()
        .map(|applied| {
            (
                applied.step_index,
                applied.move_label,
                applied.selected_candidate_index,
                applied.score_after,
            )
        })
        .collect()
}

fn assert_shadows_follow_routes(plan: &RoutePlan) {
    for route in &plan.routes {
        for (position, &visit_id) in route.visits.iter().enumerate() {
            let visit = &plan.visits[visit_id];
            assert_eq!(visit.route, Some(route.id));
            assert_eq!(visit.index, Some(position));
            assert_eq!(
                visit.previous,
                position.checked_sub(1).map(|prev| route.visits[prev])
            );
            assert_eq!(visit.next, route.visits.get(position + 1).copied());
        }
    }
}

#[test]
fn list_trajectory_and_shadows_are_identical_across_thread_counts() {
    static MANAGER: SolverManager<RoutePlan> = SolverManager::new();

    let runs: Vec<_> = [1, 2, 4]
        .into_iter()
        .map(|move_threads| solve(&MANAGER, move_threads))
        .collect();
    let baseline = &runs[0];
    assert!(baseline.telemetry.moves_applied > 0);
    assert!(!baseline.consumed_pulls.is_empty());
    assert_shadows_follow_routes(&baseline.plan);

    for run in &runs[1..] {
        assert_eq!(
            selected_moves(&run.telemetry.applied_move_trace),
            selected_moves(&baseline.telemetry.applied_move_trace)
        );
        assert_eq!(run.consumed_pulls, baseline.consumed_pulls);
        let routes: Vec<_> = run.plan.routes.iter().map(|route| &route.visits).collect();
        let baseline_routes: Vec<_> = baseline
            .plan
            .routes
            .iter()
            .map(|route| &route.visits)
            .collect();
        assert_eq!(routes, baseline_routes);
        assert_eq!(run.plan.score, baseline.plan.score);
        assert_shadows_follow_routes(&run.plan);
    }
}

#[test]
fn incremental_score_tracks_list_shadows_read_by_element_streams() {
    static MANAGER: SolverManager<RoutePlan> = SolverManager::new();

    for move_threads in [1, 2] {
        let run = solve(&MANAGER, move_threads);
        assert_shadows_follow_routes(&run.plan);
        // Every visit is routed, so the shadow-reading hard term is zero.
        assert_eq!(
            run.plan.score.map(|score| score.hard()),
            Some(0),
            "move_threads = {move_threads}"
        );
        assert_eq!(
            run.plan.score,
            Some(solverforge::analyze(&run.plan).score),
            "move_threads = {move_threads}"
        );
    }
}
