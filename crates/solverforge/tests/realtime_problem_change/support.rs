// Shared fixture and event helpers for the real-time problem-change suites.

use std::time::{Duration, Instant};

use solverforge::prelude::{ConstraintWeights, HardSoftScore};
use solverforge::{Analyzable, SolverEvent};

use super::domain::{Phases, Plan, Resource, Task};

pub(crate) const EVENT_DEADLINE: Duration = Duration::from_secs(60);

pub(crate) fn seeded_plan(phases: Phases) -> Plan {
    Plan {
        resources: (0..4)
            .map(|idx| Resource {
                id: format!("resource-{idx}"),
            })
            .collect(),
        tasks: (0..12)
            .map(|idx| Task {
                id: format!("task-{idx}"),
                weight: (idx * 5 + 2) % 9,
                resource_idx: None,
            })
            .collect(),
        weights: ConstraintWeights::new(),
        score: None,
        phases,
    }
}

/// Reads events until `select` returns a value. Local search keeps
/// publishing progress, so a silent stream past the deadline is a failure.
pub(crate) fn next_matching<T>(
    next_event: &mut impl FnMut() -> SolverEvent<Plan>,
    mut select: impl FnMut(SolverEvent<Plan>) -> Option<T>,
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

pub(crate) fn assert_fully_assigned_and_rescored(
    solution: &Plan,
    published_best: Option<HardSoftScore>,
) {
    assert!(
        solution
            .tasks
            .iter()
            .all(|task| task.resource_idx.is_some()),
        "construction must assign every task, including added ones"
    );
    let fresh = solution.analyze().score;
    assert_eq!(solution.score, Some(fresh));
    assert_eq!(published_best, Some(fresh));
}
