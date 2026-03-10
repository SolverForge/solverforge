//! Colorful console output for solver metrics.
//!
//! Provides a custom `tracing` layer that formats solver events with colors.
//!
//! ## Log Levels
//!
//! - **INFO**: Lifecycle events (solving/phase start/end)
//! - **DEBUG**: Progress updates (1/sec with speed and score)
//! - **TRACE**: Individual step evaluations

mod banner;
mod format;
mod layer;
mod time;
mod visitor;

pub use layer::SolverConsoleLayer;

use std::sync::OnceLock;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

static INIT: OnceLock<()> = OnceLock::new();

/// Initializes the solver console output.
///
/// Safe to call multiple times - only the first call has effect.
/// Prints the SolverForge banner and sets up tracing.
pub fn init() {
    INIT.get_or_init(|| {
        banner::print_banner();

        let filter = EnvFilter::builder()
            .with_default_directive("solverforge_solver=info".parse().unwrap())
            .from_env_lossy()
            .add_directive("solverforge_dynamic=info".parse().unwrap());

        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(SolverConsoleLayer)
            .try_init();
    });
}
