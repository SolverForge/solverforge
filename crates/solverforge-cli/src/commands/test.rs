use std::process::Command;

use crate::error::{CliError, CliResult};
use crate::output;

// Wraps `cargo test` with optional passthrough arguments.
pub fn run(extra_args: &[String]) -> CliResult {
    output::print_status("test", "running cargo test");

    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    cmd.args(extra_args);

    let status = cmd.status().map_err(|e| CliError::IoError {
        context: "failed to run cargo test".to_string(),
        source: e,
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(CliError::SubprocessFailed {
            command: "cargo test".to_string(),
        })
    }
}
