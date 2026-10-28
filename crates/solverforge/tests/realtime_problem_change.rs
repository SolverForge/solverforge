use std::time::{Duration, Instant};

use solverforge::prelude::HardSoftScore;
use solverforge::{
    Analyzable, ClosureProblemChange, Director, ProblemChange, SolverEvent, SolverLifecycleState,
    SolverManager, SolverManagerError,
};

#[path = "realtime_problem_change/domain/mod.rs"]
mod domain;

use domain::{Plan, Resource, Task};

const EVENT_DEADLINE: Duration = Duration::from_secs(60);

fn seeded_plan(local_search: bool) -> Plan {
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
        score: None,
        local_search,
    }
}

/// Moves a new task into the solution; construction must assign it.
#[derive(Debug)]
struct AddTask {
    task: Task,
}

impl ProblemChange<Plan> for AddTask {
    fn apply(self: Box<Self>, score_director: &mut dyn Director<Plan>) {
        score_director.working_solution_mut().tasks.push(self.task);
    }
}

fn new_task(id: &str) -> AddTask {
    AddTask {
        task: Task {
            id: id.to_string(),
            weight: 4,
            resource_idx: None,
        },
    }
}

/// Reads events until `select` returns a value. Local search keeps
/// publishing progress, so a silent stream past the deadline is a failure.
fn next_matching<T>(
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

fn assert_fully_assigned_and_rescored(solution: &Plan, published_best: Option<HardSoftScore>) {
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

#[test]
fn mid_solve_change_is_applied_assigned_and_rescored() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan(true)).expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::BestSolution { solution, .. } => Some(solution),
        _ => None,
    });

    let id = MANAGER
        .add_problem_change(job_id, new_task("task-new"))
        .expect("a solving job accepts a problem change");
    assert_eq!(id.get(), 1);

    // The applied change is announced by progress before the changed
    // solution is published.
    let mut saw_progress = false;
    let (solution, best_score) = next_matching(&mut next_event, |event| match event {
        SolverEvent::Progress { metadata } if metadata.telemetry.problem_changes_applied == 1 => {
            saw_progress = true;
            None
        }
        SolverEvent::BestSolution { metadata, solution }
            if metadata.telemetry.problem_changes_applied == 1 =>
        {
            let phases: Vec<_> = metadata
                .telemetry
                .phases
                .iter()
                .map(|phase| phase.phase_type.as_str())
                .collect();
            assert!(
                phases.len() >= 2
                    && phases
                        .iter()
                        .filter(|&&phase| phase == "Construction Heuristic")
                        .count()
                        == 2
                    && phases.last() == Some(&"Construction Heuristic"),
                "the change restarts the phase sequence with construction: {phases:?}"
            );
            Some((solution, metadata.best_score))
        }
        _ => None,
    });
    assert!(
        saw_progress,
        "progress must report the applied change first"
    );
    assert_eq!(solution.tasks.len(), 13);
    assert!(solution.tasks.iter().any(|task| task.id == "task-new"));
    assert_fully_assigned_and_rescored(&solution, best_score);

    MANAGER.cancel(job_id).expect("cancel the running job");
    assert!(matches!(
        MANAGER.add_problem_change(job_id, new_task("too-late")),
        Err(SolverManagerError::InvalidStateTransition {
            action: "add_problem_change",
            ..
        })
    ));
    let telemetry = next_matching(&mut next_event, |event| match event {
        SolverEvent::Cancelled { metadata } => Some(metadata.telemetry),
        _ => None,
    });
    assert_eq!(telemetry.problem_changes_applied, 1);
    assert!(matches!(
        MANAGER.add_problem_change(job_id, new_task("after-cancel")),
        Err(SolverManagerError::InvalidStateTransition {
            action: "add_problem_change",
            state: SolverLifecycleState::Cancelled,
            ..
        })
    ));
    MANAGER.delete(job_id).expect("delete the cancelled job");
}

#[test]
fn paused_job_applies_queued_changes_after_resume_in_submission_order() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan(true)).expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::BestSolution { .. } => Some(()),
        _ => None,
    });
    MANAGER.pause(job_id).expect("pause the running job");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::Paused { .. } => Some(()),
        _ => None,
    });

    let first = MANAGER
        .add_problem_change(job_id, new_task("task-new"))
        .expect("a paused job accepts a problem change");
    // Renaming the added task panics unless the first change ran before it.
    let second = MANAGER
        .add_problem_change(
            job_id,
            ClosureProblemChange::<Plan, _>::new("rename added task", |director| {
                let task = director
                    .working_solution_mut()
                    .tasks
                    .iter_mut()
                    .find(|task| task.id == "task-new")
                    .expect("changes apply in submission order");
                task.id = "task-renamed".to_string();
            }),
        )
        .expect("a paused job accepts a second problem change");
    assert_eq!((first.get(), second.get()), (1, 2));

    let status = MANAGER.get_status(job_id).expect("paused status");
    assert_eq!(status.lifecycle_state, SolverLifecycleState::Paused);
    assert_eq!(status.telemetry.problem_changes_applied, 0);
    let snapshot = MANAGER
        .get_snapshot(job_id, None)
        .expect("paused job keeps its checkpoint");
    assert_eq!(
        snapshot.solution.tasks.len(),
        12,
        "a paused job is never mutated"
    );

    MANAGER.resume(job_id).expect("resume the paused job");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::Resumed { .. } => Some(()),
        _ => None,
    });
    let (solution, best_score) = next_matching(&mut next_event, |event| match event {
        SolverEvent::BestSolution { metadata, solution } => {
            let applied = metadata.telemetry.problem_changes_applied;
            assert_ne!(applied, 1, "both queued changes apply in one batch");
            if applied == 0 {
                assert_eq!(solution.tasks.len(), 12);
                return None;
            }
            Some((solution, metadata.best_score))
        }
        _ => None,
    });
    assert_eq!(solution.tasks.len(), 13);
    assert!(solution.tasks.iter().any(|task| task.id == "task-renamed"));
    assert!(solution.tasks.iter().all(|task| task.id != "task-new"));
    assert_fully_assigned_and_rescored(&solution, best_score);

    MANAGER.cancel(job_id).expect("cancel the running job");
    next_matching(&mut next_event, |event| match event {
        SolverEvent::Cancelled { .. } => Some(()),
        _ => None,
    });
    MANAGER.delete(job_id).expect("delete the cancelled job");
}

#[test]
fn completed_and_unknown_jobs_reject_problem_changes() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    assert_eq!(
        MANAGER
            .add_problem_change(99, new_task("nowhere"))
            .unwrap_err(),
        SolverManagerError::JobNotFound { job_id: 99 }
    );

    let (job_id, mut receiver) = MANAGER.solve(seeded_plan(false)).expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    let solution = next_matching(&mut next_event, |event| match event {
        SolverEvent::Completed { solution, .. } => Some(solution),
        _ => None,
    });
    assert_eq!(solution.tasks.len(), 12);

    assert_eq!(
        MANAGER
            .add_problem_change(job_id, new_task("too-late"))
            .unwrap_err(),
        SolverManagerError::InvalidStateTransition {
            job_id,
            action: "add_problem_change",
            state: SolverLifecycleState::Completed,
        }
    );
    MANAGER.delete(job_id).expect("delete the completed job");
}
