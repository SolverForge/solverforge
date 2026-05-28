use std::sync::OnceLock;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::banner;
use crate::SolverConsoleLayer;

static INIT: OnceLock<()> = OnceLock::new();

/// Initializes the solver console output.
///
/// Safe to call multiple times - only the first call has effect.
/// Prints the SolverForge banner and sets up tracing.
pub fn init() {
    INIT.get_or_init(|| {
        banner::print_banner();

        #[cfg(feature = "verbose-logging")]
        let solver_level = "solverforge_solver=debug";
        #[cfg(not(feature = "verbose-logging"))]
        let solver_level = "solverforge_solver=info";

        let rust_log = std::env::var("RUST_LOG").ok();
        let rust_log = rust_log.as_deref();

        let mut filter = EnvFilter::builder()
            .with_default_directive(solver_level.parse().unwrap())
            .from_env_lossy();

        if !rust_log_has_directive_for(rust_log, "solverforge_solver")
            && !rust_log_has_global_trace(rust_log)
        {
            filter = filter.add_directive(solver_level.parse().unwrap());
        }

        if !rust_log_has_directive_for(rust_log, "solverforge_dynamic")
            && !rust_log_has_global_trace(rust_log)
        {
            filter = filter.add_directive("solverforge_dynamic=info".parse().unwrap());
        }

        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(SolverConsoleLayer)
            .try_init();
    });
}

fn rust_log_has_directive_for(rust_log: Option<&str>, target: &str) -> bool {
    rust_log
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .filter_map(|directive| directive.split_once('=').map(|(target, _)| target.trim()))
        .any(|directive_target| {
            directive_target == target
                || directive_target
                    .strip_prefix(target)
                    .is_some_and(|rest| rest.starts_with("::"))
        })
}

fn rust_log_has_global_trace(rust_log: Option<&str>) -> bool {
    rust_log
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(|directive| directive.trim().to_ascii_lowercase())
        .any(|directive| directive == "trace")
}

#[cfg(test)]
mod tests {
    use super::{rust_log_has_directive_for, rust_log_has_global_trace};

    #[test]
    fn unrelated_rust_log_does_not_disable_solver_console_defaults() {
        let rust_log = Some("warn,imap_codec=error,imap_client=error");

        assert!(!rust_log_has_directive_for(rust_log, "solverforge_solver"));
        assert!(!rust_log_has_directive_for(rust_log, "solverforge_dynamic"));
        assert!(!rust_log_has_global_trace(rust_log));
    }

    #[test]
    fn explicit_solver_target_disables_default_solver_directive() {
        assert!(rust_log_has_directive_for(
            Some("warn,solverforge_solver=trace"),
            "solverforge_solver"
        ));
        assert!(rust_log_has_directive_for(
            Some("solverforge_solver::phase=trace"),
            "solverforge_solver"
        ));
    }

    #[test]
    fn global_trace_disables_default_solver_directives() {
        assert!(rust_log_has_global_trace(Some(
            "trace,imap_codec=error,imap_client=error"
        )));
        assert!(!rust_log_has_global_trace(Some(
            "warn,solverforge_solver=trace"
        )));
    }
}
