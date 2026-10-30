//! An invalid solver configuration fails the job with one `Failed` event
//! before any phase runs, without a panic.

use std::sync::atomic::{AtomicUsize, Ordering};

use solverforge::{SolverEvent, SolverManager};

#[path = "invalid_config_lifecycle/domain/mod.rs"]
mod domain;

use domain::{ConfigCase, Plan, Resource, Task};

static PANICS: AtomicUsize = AtomicUsize::new(0);

fn plan(case: ConfigCase) -> Plan {
    Plan {
        resources: (0..3)
            .map(|idx| Resource {
                id: format!("resource-{idx}"),
            })
            .collect(),
        tasks: (0..6)
            .map(|idx| Task {
                id: format!("task-{idx}"),
                resource_idx: None,
            })
            .collect(),
        score: None,
        case,
    }
}

/// Every lifecycle event of one job, in order.
fn events(manager: &'static SolverManager<Plan>, case: ConfigCase) -> Vec<SolverEvent<Plan>> {
    let (job_id, mut receiver) = manager.solve(plan(case)).expect("job should start");
    let mut events = Vec::new();
    while let Some(event) = receiver.blocking_recv() {
        let terminal = matches!(
            event,
            SolverEvent::Completed { .. }
                | SolverEvent::Cancelled { .. }
                | SolverEvent::Failed { .. }
        );
        events.push(event);
        if terminal {
            break;
        }
    }
    manager.delete(job_id).expect("terminal job should delete");
    events
}

fn single_failure(events: &[SolverEvent<Plan>]) -> &str {
    let [SolverEvent::Failed { error, .. }] = events else {
        panic!(
            "expected exactly one Failed event, got {} events",
            events.len()
        );
    };
    error
}

#[test]
fn invalid_configuration_fails_before_any_phase_without_panicking() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();
    std::panic::set_hook(Box::new(|_| {
        PANICS.fetch_add(1, Ordering::SeqCst);
    }));

    let events_for_zero_tabu = events(&MANAGER, ConfigCase::ZeroTabuSize);
    let error = single_failure(&events_for_zero_tabu);
    assert!(
        error.contains("phases[1].local_search.acceptor.entity_tabu_size")
            && error.contains("must be greater than 0"),
        "{error}"
    );

    let events_for_list_selector = events(&MANAGER, ConfigCase::ListSelectorOnScalarModel);
    let error = single_failure(&events_for_list_selector);
    assert!(
        error.contains("phases[1].local_search.move_selector"),
        "{error}"
    );

    let events_for_solver_limit = events(&MANAGER, ConfigCase::InvalidSolverScoreLimit);
    let error = single_failure(&events_for_solver_limit);
    assert!(
        error.contains("termination.best_score_limit") && error.contains("not-a-score"),
        "{error}"
    );
    assert!(!error.contains("phases["), "{error}");

    let events_for_phase_limit = events(&MANAGER, ConfigCase::InvalidPhaseScoreLimit);
    let error = single_failure(&events_for_phase_limit);
    assert!(
        error.contains("phases[1].local_search.termination.best_score_limit")
            && error.contains("0hard/-5sotf"),
        "{error}"
    );

    for (case, key) in [
        (
            ConfigCase::ZeroConstructionValueCandidateLimit,
            "phases[0].construction.value_candidate_limit",
        ),
        (
            ConfigCase::ZeroChangeValueCandidateLimit,
            "phases[1].local_search.move_selector.value_candidate_limit",
        ),
        (
            ConfigCase::PillarChangeMinimumAboveMaximum,
            "phases[1].local_search.move_selector.minimum_sub_pillar_size",
        ),
        (
            ConfigCase::PillarSwapMinimumAboveMaximum,
            "phases[1].local_search.move_selector.minimum_sub_pillar_size",
        ),
    ] {
        let case_events = events(&MANAGER, case);
        let error = single_failure(&case_events);
        assert!(error.contains(key), "{case:?}: {error}");
    }

    let full_pillars = events(&MANAGER, ConfigCase::FullPillarChange);
    assert!(
        matches!(full_pillars.last(), Some(SolverEvent::Completed { .. })),
        "0/0 sub-pillar sizes (full pillars only) should complete"
    );

    let valid = events(&MANAGER, ConfigCase::Valid);
    assert!(
        matches!(valid.last(), Some(SolverEvent::Completed { .. })),
        "valid configuration should complete"
    );

    let _ = std::panic::take_hook();
    assert_eq!(PANICS.load(Ordering::SeqCst), 0, "no thread may panic");
}
