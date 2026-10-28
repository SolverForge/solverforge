// Facade-level runtime constraint weights on a macro-generated model.

#[path = "constraint_weights/mod.rs"]
mod domain;

pub use domain::{Plan, Task, Worker, COST, PREFERRED};

use solverforge::prelude::*;
use solverforge::{
    analyze, ConstraintWeightError, SolverEvent, SolverLifecycleState, SolverManager,
    MAX_CONSTRAINT_WEIGHT_MULTIPLIER,
};

const WORKER_COSTS: [i64; 3] = [5, 1, 3];

fn plan(weights: ConstraintWeights) -> Plan {
    Plan {
        workers: WORKER_COSTS
            .iter()
            .enumerate()
            .map(|(id, &cost)| Worker { id, cost })
            .collect(),
        tasks: (0..6)
            .map(|id| Task {
                id,
                preferred: id % 3,
                worker: None,
            })
            .collect(),
        weights,
        score: None,
    }
}

fn solve(manager: &'static SolverManager<Plan>, plan: Plan) -> Result<Plan, String> {
    let (job_id, mut receiver) = manager.solve(plan).expect("job should start");
    let mut outcome = None;
    while let Some(event) = receiver.blocking_recv() {
        match event {
            SolverEvent::Completed { metadata, solution } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Completed);
                outcome = Some(Ok(solution));
                break;
            }
            SolverEvent::Failed { metadata, error } => {
                assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Failed);
                outcome = Some(Err(error));
                break;
            }
            _ => {}
        }
    }
    manager.delete(job_id).expect("delete terminal job");
    outcome.expect("solve should reach a terminal event")
}

fn assigned_workers(plan: &Plan) -> Vec<Option<usize>> {
    plan.tasks.iter().map(|task| task.worker).collect()
}

#[test]
fn zero_weight_changes_the_solved_assignment() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let coded = solve(&MANAGER, plan(ConstraintWeights::new())).expect("coded solve");
    assert_eq!(
        assigned_workers(&coded),
        (0..6).map(|id| Some(id % 3)).collect::<Vec<_>>(),
        "coded weights prefer each task's preferred worker"
    );

    let mut weights = ConstraintWeights::new();
    weights.set(PREFERRED, 0);
    let disabled = solve(&MANAGER, plan(weights)).expect("weighted solve");
    assert_eq!(
        assigned_workers(&disabled),
        vec![Some(1); 6],
        "disabling the preference leaves only the cheapest worker"
    );
    assert_eq!(disabled.score, Some(HardSoftScore::of_soft(-6)));
}

#[test]
fn unknown_constraint_weight_fails_the_solve_before_it_starts() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let mut weights = ConstraintWeights::new();
    weights.set("Prefered worker", 2);
    let (job_id, mut receiver) = MANAGER.solve(plan(weights)).expect("job should start");
    match receiver.blocking_recv().expect("terminal event") {
        SolverEvent::Failed { metadata, error } => {
            assert_eq!(metadata.lifecycle_state, SolverLifecycleState::Failed);
            assert!(
                error.contains("unknown constraint \"Prefered worker\""),
                "{error}"
            );
            assert!(
                error.contains("did you mean \"Preferred worker\""),
                "{error}"
            );
        }
        other => panic!("expected an immediate failure, got {other:?}"),
    }
    MANAGER.delete(job_id).expect("delete failed job");
}

#[test]
fn generated_validator_checks_weights_without_solving() {
    let mut weights = ConstraintWeights::new();
    weights.set(COST, 4);
    assert_eq!(Plan::validate_constraint_weights(&weights), Ok(()));

    weights.set("Worker costs", 1);
    let error = Plan::validate_constraint_weights(&weights).unwrap_err();
    assert!(matches!(
        error,
        ConstraintWeightError::UnknownConstraint { ref name, .. } if name == "Worker costs"
    ));

    let mut too_large = ConstraintWeights::new();
    too_large.set(COST, MAX_CONSTRAINT_WEIGHT_MULTIPLIER + 1);
    assert!(matches!(
        Plan::validate_constraint_weights(&too_large),
        Err(ConstraintWeightError::MultiplierTooLarge { .. })
    ));
}

#[test]
fn analysis_reports_weighted_scores_and_multipliers() {
    let mut weights = ConstraintWeights::new();
    weights.set(PREFERRED, 0).set(COST, 3);
    let mut solution = plan(weights);
    for task in &mut solution.tasks {
        task.worker = Some(task.preferred);
    }

    let analysis = analyze(&solution);
    let preferred = &analysis.constraints[0];
    let cost = &analysis.constraints[1];
    assert_eq!(preferred.name, PREFERRED);
    assert_eq!(preferred.weight_multiplier, 0);
    assert_eq!(preferred.match_count, 6);
    assert_eq!(preferred.score, HardSoftScore::ZERO);
    assert_eq!(cost.weight_multiplier, 3);
    assert_eq!(cost.score, HardSoftScore::of_soft(-3 * 2 * (5 + 1 + 3)));
    assert_eq!(analysis.score, cost.score);
}

#[test]
#[should_panic(expected = "unknown constraint")]
fn analysis_rejects_unknown_constraint_weights() {
    let mut weights = ConstraintWeights::new();
    weights.set("Nope", 1);
    let _ = analyze(&plan(weights));
}
