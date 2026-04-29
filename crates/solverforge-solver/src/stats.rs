mod phase;
mod solver;
mod telemetry;

pub use phase::PhaseStats;
pub use solver::SolverStats;
pub(crate) use telemetry::{format_duration, whole_units_per_second};
pub use telemetry::{SelectorTelemetry, SolverTelemetry, Throughput};

#[cfg(test)]
mod tests;
