use std::process::Command;

use crate::error::{CliError, CliResult};
use crate::output;

pub fn run(port: u16, debug: bool) -> CliResult {
    let mode = if debug { "debug" } else { "release" };

    output::print_status("start", &format!("SolverForge server ({})", mode));

    if debug {
        output::print_dim("  Compiling in debug mode... (this may take a moment on first run)");
    } else {
        output::print_dim("  Compiling in release mode... (this may take a minute on first run)");
    }

    let mut args = vec!["run"];
    if !debug {
        args.push("--release");
    }

    // Set PORT env var for the server to pick up
    let status = Command::new("cargo")
        .args(&args)
        .env("PORT", port.to_string())
        .status()
        .map_err(|e| CliError::IoError {
            context: "failed to run cargo".to_string(),
            source: e,
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(CliError::SubprocessFailed {
            command: format!("cargo run --{}", mode),
        })
    }
}
