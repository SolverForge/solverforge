// Mid-solve constraint weight changes across the director's weight modes.
//
// A job whose weights are all `1` scores on the unweighted director; a
// problem change that installs other multipliers switches it to the weighted
// director, and a change back to all-`1` weights switches it back. Each switch
// must keep the job's state: statistics, phase history and lifecycle.

use std::time::Instant;

use solverforge::prelude::{ConstraintWeights, HardSoftScore};
use solverforge::{
    Analyzable, ClosureProblemChange, ProblemChange, SolverEvent, SolverManager, SolverTelemetry,
};

#[path = "realtime_problem_change/domain/mod.rs"]
mod domain;
#[path = "realtime_problem_change/support.rs"]
mod support;

use domain::{Phases, Plan};
use support::{assert_fully_assigned_and_rescored, next_matching, seeded_plan, EVENT_DEADLINE};

fn weights_of(entries: &[(&str, u32)]) -> ConstraintWeights {
    let mut weights = ConstraintWeights::new();
    for &(name, multiplier) in entries {
        weights.set(name, multiplier);
    }
    weights
}

/// Replaces the solution's constraint weights; the solver rescores under them.
fn set_weights(entries: &[(&str, u32)]) -> impl ProblemChange<Plan> {
    let weights = weights_of(entries);
    ClosureProblemChange::<Plan, _>::new("set constraint weights", move |director| {
        director.working_solution_mut().weights = weights.clone();
    })
}

const MIXED: &[(&str, u32)] = &[("shared resource", 0), ("placement cost", 3)];
const UNIT: &[(&str, u32)] = &[("shared resource", 1), ("placement cost", 1)];

/// The first published best solution after `applied` problem changes, with
/// its telemetry.
fn best_after(
    next_event: &mut impl FnMut() -> SolverEvent<Plan>,
    applied: u64,
) -> (Plan, Option<HardSoftScore>, SolverTelemetry) {
    next_matching(next_event, |event| match event {
        SolverEvent::BestSolution { metadata, solution }
            if metadata.telemetry.problem_changes_applied == applied =>
        {
            Some((solution, metadata.best_score, metadata.telemetry))
        }
        SolverEvent::Completed { .. } | SolverEvent::Cancelled { .. } => {
            panic!("the job ended before publishing a best solution after {applied} changes")
        }
        _ => None,
    })
}

/// Statistics and phase history continue across a director switch instead of
/// restarting with the new director.
fn assert_continued(before: &SolverTelemetry, after: &SolverTelemetry) {
    assert!(after.elapsed >= before.elapsed, "the solve clock restarted");
    assert!(
        after.step_count >= before.step_count,
        "step count restarted"
    );
    assert!(
        after.moves_evaluated >= before.moves_evaluated,
        "move counters restarted"
    );
    assert!(
        after.score_calculations > before.score_calculations,
        "score calculation count restarted"
    );
    let phase_types = |telemetry: &SolverTelemetry| {
        telemetry
            .phases
            .iter()
            .map(|phase| phase.phase_type.clone())
            .collect::<Vec<_>>()
    };
    let (before_phases, after_phases) = (phase_types(before), phase_types(after));
    assert!(
        after_phases.starts_with(&before_phases),
        "the phase history restarted: {before_phases:?} -> {after_phases:?}"
    );
}

fn shared_resource(solution: &Plan) -> (u32, HardSoftScore) {
    let analysis = solution.analyze();
    let shared = analysis
        .constraints
        .iter()
        .find(|constraint| constraint.name == "shared resource")
        .expect("shared resource is analyzed");
    (shared.weight_multiplier, shared.score)
}

fn cancel(
    manager: &'static SolverManager<Plan>,
    job_id: usize,
    next_event: &mut impl FnMut() -> SolverEvent<Plan>,
) {
    manager.cancel(job_id).expect("cancel the running job");
    next_matching(next_event, |event| match event {
        SolverEvent::Cancelled { .. } => Some(()),
        SolverEvent::Completed { .. } => panic!("a cancelled job must not also complete"),
        _ => None,
    });
    manager.delete(job_id).expect("delete the cancelled job");
}

#[test]
fn mid_solve_weight_change_rescores_under_the_new_multipliers() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER
        .solve(seeded_plan(Phases::ConstructionAndSearch))
        .expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    let (_, _, before) = best_after(&mut next_event, 0);

    MANAGER
        .add_problem_change(job_id, set_weights(MIXED))
        .expect("a solving job accepts a weight change");
    let (solution, best_score, after) = best_after(&mut next_event, 1);

    assert_eq!(solution.weights.get("shared resource"), Some(0));
    assert_eq!(solution.weights.get("placement cost"), Some(3));
    assert_fully_assigned_and_rescored(&solution, best_score);
    assert_eq!(shared_resource(&solution), (0, HardSoftScore::ZERO));
    assert_continued(&before, &after);

    cancel(&MANAGER, job_id, &mut next_event);
}

#[test]
fn weighted_job_switches_to_unit_weights_and_back() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let mut plan = seeded_plan(Phases::ConstructionAndSearch);
    plan.weights = weights_of(MIXED);
    let (job_id, mut receiver) = MANAGER.solve(plan).expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    let (solution, best_score, first) = best_after(&mut next_event, 0);
    assert_fully_assigned_and_rescored(&solution, best_score);
    assert_eq!(shared_resource(&solution), (0, HardSoftScore::ZERO));

    // All-`1` weights: the job continues on the unweighted director.
    MANAGER
        .add_problem_change(job_id, set_weights(UNIT))
        .expect("a solving job accepts a weight change");
    let (solution, best_score, unit) = best_after(&mut next_event, 1);
    assert_fully_assigned_and_rescored(&solution, best_score);
    let (multiplier, _) = shared_resource(&solution);
    assert_eq!(multiplier, 1);
    assert_continued(&first, &unit);

    // And back to the weighted director.
    MANAGER
        .add_problem_change(job_id, set_weights(MIXED))
        .expect("a solving job accepts a second weight change");
    let (solution, best_score, mixed) = best_after(&mut next_event, 2);
    assert_fully_assigned_and_rescored(&solution, best_score);
    assert_eq!(shared_resource(&solution), (0, HardSoftScore::ZERO));
    assert_continued(&unit, &mixed);

    cancel(&MANAGER, job_id, &mut next_event);
}

/// Runs `plan` to completion and returns the completed solution.
fn completed(manager: &'static SolverManager<Plan>, plan: Plan) -> (Plan, Option<HardSoftScore>) {
    let (job_id, mut receiver) = manager.solve(plan).expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    let completed = next_matching(&mut next_event, |event| match event {
        SolverEvent::Completed { metadata, solution } => Some((solution, metadata.best_score)),
        _ => None,
    });
    manager.delete(job_id).expect("delete the completed job");
    completed
}

#[test]
fn job_starting_with_non_unit_weights_completes_under_them() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let mut plan = seeded_plan(Phases::ConstructionOnly);
    plan.weights = weights_of(MIXED);
    let (solution, best_score) = completed(&MANAGER, plan);
    assert_fully_assigned_and_rescored(&solution, best_score);
    assert_eq!(shared_resource(&solution), (0, HardSoftScore::ZERO));
}

#[test]
fn job_with_unit_weights_completes_on_coded_weights() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let mut plan = seeded_plan(Phases::Default);
    plan.weights = weights_of(UNIT);
    let (solution, best_score) = completed(&MANAGER, plan);
    assert_fully_assigned_and_rescored(&solution, best_score);
    assert_eq!(shared_resource(&solution).0, 1);
}

#[test]
fn mid_solve_unknown_constraint_weight_fails_the_job() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let (job_id, mut receiver) = MANAGER
        .solve(seeded_plan(Phases::ConstructionAndSearch))
        .expect("job starts");
    let mut next_event = || receiver.blocking_recv().expect("event stream stays open");
    best_after(&mut next_event, 0);

    MANAGER
        .add_problem_change(job_id, set_weights(&[("shared resorce", 2)]))
        .expect("the change is queued; validation runs when it applies");
    let deadline = Instant::now() + EVENT_DEADLINE;
    let error = loop {
        assert!(Instant::now() < deadline, "the job did not fail");
        if let SolverEvent::Failed { error, .. } = next_event() {
            break error;
        }
    };
    assert!(
        error.contains("unknown constraint \"shared resorce\""),
        "{error}"
    );
    assert!(
        error.contains("did you mean \"shared resource\""),
        "{error}"
    );
    MANAGER.delete(job_id).expect("delete the failed job");
}
