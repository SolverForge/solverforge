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

        let filter = EnvFilter::builder()
            .with_default_directive(solver_level.parse().unwrap())
            .from_env_lossy()
            .add_directive(solver_level.parse().unwrap())
            .add_directive("solverforge_dynamic=info".parse().unwrap());

        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(SolverConsoleLayer)
            .try_init();
    });
}
