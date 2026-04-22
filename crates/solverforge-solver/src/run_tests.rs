use super::{load_solver_config_from, log_solve_start};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
