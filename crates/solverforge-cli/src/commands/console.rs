use crate::error::CliResult;
use crate::output;

// The interactive console is not yet available as a standalone command.
// The solver runtime runs inside the generated project, not inside the CLI binary.
// Use the solver directly via `cargo run` or `solverforge server` in your project.
pub fn run() -> CliResult {
    output::print_heading("SolverForge Console");
    println!();
    println!("  The interactive console is not yet available in this version.");
    println!();
    println!("  To work with the solver interactively, use one of:");
    println!("    solverforge server          - start the HTTP server with live scoring");
    println!("    cargo run                   - run the solver directly");
    println!("    solverforge test -- --nocapture  - run tests with output");
    println!();
    Ok(())
}
