// Integration tests for project scaffolding.
//
// Tests marked `#[ignore]` invoke `cargo check` inside a temp directory which requires a full
// Rust toolchain and network access (to fetch crate dependencies). Run them explicitly with:
//
//   cargo test -p solverforge-cli -- --ignored

use std::path::PathBuf;
use std::process::Command;

// Locate the built CLI binary. Falls back to a bare `solverforge` on PATH.
fn cli_bin() -> PathBuf {
    // Prefer the binary built by the current workspace build.
    let mut path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // The test binary lives in deps/; the actual binary is one level up.
    if path.ends_with("deps") {
        path = path.parent().unwrap().to_path_buf();
    }

    let candidate = path.join("solverforge");
    if candidate.exists() {
        candidate
    } else {
        PathBuf::from("solverforge")
    }
}

// Scaffold a basic project and verify the expected files are created.
#[test]
fn test_new_basic_creates_project_files() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let project_name = "test_basic_project";

    let status = Command::new(cli_bin())
        .args([
            "new",
            project_name,
            "--basic",
            "--skip-git",
            "--skip-readme",
            "--quiet",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("failed to run solverforge new");

    assert!(status.success(), "solverforge new --basic failed");

    let project_dir = tmp.path().join(project_name);
    assert!(project_dir.exists(), "project directory not created");
    assert!(
        project_dir.join("Cargo.toml").exists(),
        "Cargo.toml missing"
    );
    assert!(project_dir.join("src").exists(), "src/ directory missing");
    assert!(
        project_dir.join(".gitignore").exists(),
        ".gitignore missing"
    );
    assert!(
        project_dir.join("solver.toml").exists(),
        "solver.toml missing"
    );
}

// Scaffold an employee-scheduling project and verify the domain files are created.
#[test]
fn test_new_employee_scheduling_creates_domain() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let project_name = "test_emp_project";

    let status = Command::new(cli_bin())
        .args([
            "new",
            project_name,
            "--basic=employee-scheduling",
            "--skip-git",
            "--skip-readme",
            "--quiet",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("failed to run solverforge new");

    assert!(
        status.success(),
        "solverforge new --basic=employee-scheduling failed"
    );

    let project_dir = tmp.path().join(project_name);
    let domain_dir = project_dir.join("src").join("domain");
    assert!(domain_dir.exists(), "src/domain/ missing");

    let constraints_dir = project_dir.join("src").join("constraints");
    assert!(constraints_dir.exists(), "src/constraints/ missing");
}

// Scaffold a vehicle-routing project and verify the domain files are created.
#[test]
fn test_new_vehicle_routing_creates_domain() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let project_name = "test_vr_project";

    let status = Command::new(cli_bin())
        .args([
            "new",
            project_name,
            "--list=vehicle-routing",
            "--skip-git",
            "--skip-readme",
            "--quiet",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("failed to run solverforge new");

    assert!(
        status.success(),
        "solverforge new --list=vehicle-routing failed"
    );

    let project_dir = tmp.path().join(project_name);
    assert!(
        project_dir.join("Cargo.toml").exists(),
        "Cargo.toml missing"
    );
}

// Full cargo check on a scaffolded basic project.
// Requires network access and a full Rust toolchain.
// Run with: cargo test -p solverforge-cli -- --ignored
#[test]
#[ignore = "invokes cargo check in a temp dir; requires network + toolchain; run with --ignored"]
fn test_new_basic_cargo_check_passes() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let project_name = "test_cargo_check_basic";

    let scaffold_status = Command::new(cli_bin())
        .args([
            "new",
            project_name,
            "--basic",
            "--skip-git",
            "--skip-readme",
            "--quiet",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("failed to run solverforge new");

    assert!(scaffold_status.success(), "scaffolding failed");

    let project_dir = tmp.path().join(project_name);
    let check_status = Command::new("cargo")
        .arg("check")
        .current_dir(&project_dir)
        .status()
        .expect("failed to run cargo check");

    assert!(
        check_status.success(),
        "cargo check failed on scaffolded basic project"
    );
}

// Full cargo check on a scaffolded employee-scheduling project.
// Run with: cargo test -p solverforge-cli -- --ignored
#[test]
#[ignore = "invokes cargo check in a temp dir; requires network + toolchain; run with --ignored"]
fn test_new_employee_scheduling_cargo_check_passes() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let project_name = "test_cargo_check_employee";

    let scaffold_status = Command::new(cli_bin())
        .args([
            "new",
            project_name,
            "--basic=employee-scheduling",
            "--skip-git",
            "--skip-readme",
            "--quiet",
        ])
        .current_dir(tmp.path())
        .status()
        .expect("failed to run solverforge new");

    assert!(scaffold_status.success(), "scaffolding failed");

    let project_dir = tmp.path().join(project_name);
    let check_status = Command::new("cargo")
        .arg("check")
        .current_dir(&project_dir)
        .status()
        .expect("failed to run cargo check");

    assert!(
        check_status.success(),
        "cargo check failed on employee-scheduling project"
    );
}
