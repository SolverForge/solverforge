use std::time::{Duration, Instant};

use solverforge::{Analyzable, ClosureProblemChange, SolverEvent, SolverManager};

#[path = "realtime_list_problem_change/domain/mod.rs"]
mod domain;

use domain::{Route, RoutePlan, Visit};

const EVENT_DEADLINE: Duration = Duration::from_secs(60);

fn visit(id: usize) -> Visit {
    Visit {
        id,
        demand: (id as i64 * 5 + 2) % 9,
        route: None,
        index: None,
        previous: None,
        next: None,
    }
}

fn seeded_plan() -> RoutePlan {
    RoutePlan {
        routes: (0..3)
            .map(|id| Route {
                id,
                visits: Vec::new(),
            })
            .collect(),
        visits: (0..12).map(visit).collect(),
        score: None,
    }
}

fn next_matching<T>(
    next_event: &mut impl FnMut() -> SolverEvent<RoutePlan>,
    mut select: impl FnMut(SolverEvent<RoutePlan>) -> Option<T>,
) -> T {
    let deadline = Instant::now() + EVENT_DEADLINE;
    loop {
        assert!(Instant::now() < deadline, "expected event did not arrive");
        let event = next_event();
        if let SolverEvent::Failed { error, .. } = &event {
            panic!("real-time solve failed: {error}");
        }
        if let Some(selected) = select(event) {
            return selected;
        }
    }
}

fn assert_every_visit_routed_once(plan: &RoutePlan) {
    let mut seen = vec![0usize; plan.visits.len()];
    for route in &plan.routes {
        for (position, &visit_id) in route.visits.iter().enumerate() {
            seen[visit_id] += 1;
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
    assert!(
        seen.iter().all(|&count| count == 1),
        "every visit, including added ones, is routed exactly once: {seen:?}"
    );
}

#[test]
fn list_variable_job_routes_an_element_added_mid_solve() {
    static MANAGER: SolverManager<RoutePlan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan()).expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::BestSolution { .. } => Some(()),
        _ => None,
    });

    let id = MANAGER
        .add_problem_change(
            job_id,
            ClosureProblemChange::<RoutePlan, _>::new("add visit", |director| {
                let visits = &mut director.working_solution_mut().visits;
                visits.push(visit(visits.len()));
            }),
        )
        .expect("a solving job accepts a problem change");
    assert_eq!(id.get(), 1);

    let (solution, best_score) = next_matching(&mut next_event, |event| match event {
        SolverEvent::BestSolution { metadata, solution }
            if metadata.telemetry.problem_changes_applied == 1 =>
        {
            Some((solution, metadata.best_score))
        }
        _ => None,
    });
    assert_eq!(solution.visits.len(), 13);
    assert_every_visit_routed_once(&solution);
    let fresh = solution.analyze().score;
    assert_eq!(best_score, Some(fresh));
    assert_eq!(solution.score, Some(fresh));

    MANAGER.cancel(job_id).expect("cancel the running job");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::Cancelled { .. } => Some(()),
        _ => None,
    });
    MANAGER.delete(job_id).expect("delete the cancelled job");
}
