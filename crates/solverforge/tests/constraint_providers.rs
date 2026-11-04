//! `[score_director]`: `constraint_provider` selects the constraint set a
//! solve scores with, and `constraint_match_enabled` gates snapshot analysis.

use solverforge::prelude::*;
use solverforge::{
    RuntimeBuildError, SolverConfig, SolverEvent, SolverManager, SolverManagerError,
};

#[path = "constraint_providers/domain/mod.rs"]
mod domain;

use domain::{Plan, CONSTRUCTION_TOML};

fn config(score_director: &str) -> SolverConfig {
    SolverConfig::from_toml_str(&format!("{CONSTRUCTION_TOML}\n{score_director}"))
        .expect("the test config parses")
}

const RELAXED: &str = "[score_director]\nconstraint_provider = \"relaxed_constraints\"\n";
const DEFAULT_BY_NAME: &str = "[score_director]\nconstraint_provider = \"define_constraints\"\n";
const DISABLED: &str = "[score_director]\nconstraint_match_enabled = false\n";

#[test]
fn the_selected_provider_scores_the_solve() {
    // First fit puts every task on resource 0, which costs 1 per task under
    // the default constraints and nothing under the relaxed ones.
    let default = Plan::new("").solve_with_config(config("")).expect("solves");
    assert_eq!(default.score, Some(HardSoftScore::of(0, -4)));

    let by_name = Plan::new("")
        .solve_with_config(config(DEFAULT_BY_NAME))
        .expect("solves");
    assert_eq!(by_name.score, Some(HardSoftScore::of(0, -4)));

    let relaxed = Plan::new("")
        .solve_with_config(config(RELAXED))
        .expect("solves");
    assert_eq!(relaxed.score, Some(HardSoftScore::of(0, 0)));
}

#[test]
fn an_unknown_provider_fails_before_solving() {
    let error = Plan::new("")
        .solve_with_config(config(
            "[score_director]\nconstraint_provider = \"crate::missing\"\n",
        ))
        .expect_err("an unknown provider is rejected");
    assert_eq!(
        error,
        RuntimeBuildError::UnknownConstraintProvider {
            requested: "crate::missing".to_string(),
            declared: vec![
                "define_constraints".to_string(),
                "relaxed_constraints".to_string(),
            ],
        }
    );
}

/// Solves `plan` to completion and returns its job id.
fn solve_to_completion(manager: &'static SolverManager<Plan>, plan: Plan) -> usize {
    let (job_id, mut receiver) = manager.solve(plan).expect("job starts");
    loop {
        match receiver.blocking_recv().expect("event stream stays open") {
            SolverEvent::Completed { .. } => return job_id,
            SolverEvent::Failed { error, .. } => panic!("solve failed: {error}"),
            _ => {}
        }
    }
}

fn analyzed_constraint_names(manager: &SolverManager<Plan>, job_id: usize) -> Vec<String> {
    manager
        .analyze_snapshot(job_id, None)
        .expect("analysis is enabled")
        .analysis
        .constraints
        .into_iter()
        .map(|constraint| constraint.name)
        .collect()
}

#[test]
fn snapshot_analysis_uses_the_selected_provider() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let default_job = solve_to_completion(&MANAGER, Plan::new(""));
    assert_eq!(
        analyzed_constraint_names(&MANAGER, default_job),
        ["unassigned task", "placement cost"]
    );

    let relaxed_job = solve_to_completion(&MANAGER, Plan::new(RELAXED));
    assert_eq!(
        analyzed_constraint_names(&MANAGER, relaxed_job),
        ["unassigned task (relaxed)"]
    );
    let analysis = MANAGER.analyze_snapshot(relaxed_job, None).unwrap();
    assert_eq!(analysis.analysis.score, HardSoftScore::of(0, 0));
}

#[test]
fn disabled_constraint_matching_rejects_snapshot_analysis() {
    static MANAGER: SolverManager<Plan> = SolverManager::new();

    let job_id = solve_to_completion(&MANAGER, Plan::new(DISABLED));
    assert!(MANAGER.get_snapshot(job_id, None).is_ok());
    assert_eq!(
        MANAGER.analyze_snapshot(job_id, None).unwrap_err(),
        SolverManagerError::SnapshotAnalysisDisabled { job_id }
    );

    // The slot's next job binds its own analysis.
    MANAGER
        .delete(job_id)
        .expect("a completed job can be deleted");
    let next = solve_to_completion(&MANAGER, Plan::new(""));
    assert_eq!(
        analyzed_constraint_names(&MANAGER, next),
        ["unassigned task", "placement cost"]
    );
}
