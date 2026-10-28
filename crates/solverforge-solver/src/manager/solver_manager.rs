/* SolverManager for retained async job lifecycle management.

Provides the high-level API for:
- Starting retained solve jobs that stream lifecycle events
- Tracking authoritative job lifecycle state
- Pausing and resuming jobs at exact runtime-safe boundaries
- Queuing real-time problem changes onto running or paused jobs
- Cancelling and deleting retained jobs
- Retrieving snapshot-bound solutions and score analysis
*/

mod manager;
mod problem_change;
mod runtime;
mod slot;
mod types;

#[allow(unused_imports)]
pub use manager::{Solvable, SolverManager, MAX_JOBS};
pub use problem_change::ProblemChangeId;
pub use runtime::{SolverPanicPayload, SolverRuntime};
pub use types::{
    SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverManagerError, SolverSnapshot,
    SolverSnapshotAnalysis, SolverStatus, SolverTelemetryDetail, SolverTerminalReason,
};
