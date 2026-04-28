use super::{build_termination, load_solver_config_from, log_solve_start, AnyTermination};
use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SoftScore;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct TestSolution {
    score: Option<SoftScore>,
}

impl PlanningSolution for TestSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn temp_config_path() -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir()
        .join(format!(
            "solverforge-run-tests-{}-{suffix}",
            std::process::id()
        ))
        .join("solver.toml")
}

#[test]
fn load_solver_config_from_preserves_file_settings() {
    let path = temp_config_path();
    let parent = path.parent().expect("temp file should have a parent");
    fs::create_dir_all(parent).expect("temp directory should be created");
    fs::write(
        &path,
        r#"
random_seed = 41

[termination]
seconds_spent_limit = 5

[[phases]]
type = "construction_heuristic"
construction_heuristic_type = "first_fit"
"#,
    )
    .expect("solver.toml should be written");

    let config = load_solver_config_from(&path);

    assert_eq!(config.random_seed, Some(41));
    assert_eq!(config.time_limit(), Some(Duration::from_secs(5)));
    assert_eq!(config.phases.len(), 1);

    fs::remove_dir_all(parent).expect("temp directory should be removed");
}

#[test]
fn build_termination_preserves_missing_time_limit_as_unlimited() {
    let config = SolverConfig::default();
    let (termination, time_limit) = build_termination::<TestSolution, ()>(&config, 180);

    assert!(matches!(termination, AnyTermination::None(_)));
    assert_eq!(time_limit, None);
}

#[test]
fn build_termination_returns_fallback_time_for_best_score_limit() {
    let mut config = SolverConfig::default();
    config.termination = Some(solverforge_config::TerminationConfig {
        best_score_limit: Some("0".to_string()),
        ..Default::default()
    });

    let (termination, time_limit) = build_termination::<TestSolution, ()>(&config, 180);

    assert!(matches!(termination, AnyTermination::WithBestScore(_)));
    assert_eq!(time_limit, Some(Duration::from_secs(180)));
}

#[test]
fn build_termination_returns_fallback_time_for_step_limit() {
    let mut config = SolverConfig::default();
    config.termination = Some(solverforge_config::TerminationConfig {
        step_count_limit: Some(10),
        ..Default::default()
    });

    let (termination, time_limit) = build_termination::<TestSolution, ()>(&config, 180);

    assert!(matches!(termination, AnyTermination::WithStepCount(_)));
    assert_eq!(time_limit, Some(Duration::from_secs(180)));
}

#[test]
fn build_termination_returns_fallback_time_for_unimproved_step_limit() {
    let mut config = SolverConfig::default();
    config.termination = Some(solverforge_config::TerminationConfig {
        unimproved_step_count_limit: Some(10),
        ..Default::default()
    });

    let (termination, time_limit) = build_termination::<TestSolution, ()>(&config, 180);

    assert!(matches!(termination, AnyTermination::WithUnimprovedStep(_)));
    assert_eq!(time_limit, Some(Duration::from_secs(180)));
}

#[test]
fn build_termination_returns_fallback_time_for_unimproved_time_limit() {
    let mut config = SolverConfig::default();
    config.termination = Some(solverforge_config::TerminationConfig {
        unimproved_seconds_spent_limit: Some(10),
        ..Default::default()
    });

    let (termination, time_limit) = build_termination::<TestSolution, ()>(&config, 180);

    assert!(matches!(termination, AnyTermination::WithUnimprovedTime(_)));
    assert_eq!(time_limit, Some(Duration::from_secs(180)));
}

#[test]
fn build_termination_explicit_time_overrides_fallback() {
    let mut config = SolverConfig::default();
    config.termination = Some(solverforge_config::TerminationConfig {
        step_count_limit: Some(10),
        seconds_spent_limit: Some(5),
        ..Default::default()
    });

    let (termination, time_limit) = build_termination::<TestSolution, ()>(&config, 180);

    assert!(matches!(termination, AnyTermination::WithStepCount(_)));
    assert_eq!(time_limit, Some(Duration::from_secs(5)));
}

#[test]
fn log_solve_start_rejects_missing_scale() {
    let panic = std::panic::catch_unwind(|| log_solve_start(4, None, None))
        .expect_err("missing solve scale must panic");
    let message = if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        panic!("unexpected panic payload");
    };
    assert!(message.contains("requires exactly one solve scale"));
}

#[test]
fn log_solve_start_rejects_ambiguous_scale() {
    let panic = std::panic::catch_unwind(|| log_solve_start(4, Some(3), Some(9)))
        .expect_err("ambiguous solve scale must panic");
    let message = if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else {
        panic!("unexpected panic payload");
    };
    assert!(message.contains("requires exactly one solve scale"));
}
