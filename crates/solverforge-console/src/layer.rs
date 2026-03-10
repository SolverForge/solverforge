// Tracing layer that routes solver events to the console formatter.

use crate::format::format_event;
use crate::visitor::EventVisitor;
use std::io::{self, Write};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

/// A tracing layer that formats solver events with colors.
pub struct SolverConsoleLayer;

impl<S: Subscriber> Layer<S> for SolverConsoleLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let target = metadata.target();

        // Accept events from solver modules
        if !target.starts_with("solverforge_solver")
            && !target.starts_with("solverforge_dynamic")
            && !target.starts_with("solverforge_py")
            && !target.starts_with("solverforge::")
        {
            return;
        }

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let level = *metadata.level();
        let output = format_event(&visitor, level);
        if !output.is_empty() {
            let _ = writeln!(io::stdout(), "{}", output);
        }
    }
}
