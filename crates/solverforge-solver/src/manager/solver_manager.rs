/* SolverManager for retained async job lifecycle management.

Provides the high-level API for:
- Starting retained solve jobs that stream lifecycle events
- Tracking authoritative job lifecycle state
- Pausing and resuming jobs at exact runtime-safe boundaries
- Cancelling and deleting retained jobs
- Retrieving snapshot-bound solutions and score analysis
*/

mod manager;
mod runtime;
mod slot;
mod types;

#[allow(unused_imports)]
pub use manager::{Solvable, SolverManager, MAX_JOBS};
pub use runtime::SolverRuntime;
pub use types::{
    SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverManagerError, SolverSnapshot,
    SolverSnapshotAnalysis, SolverStatus, SolverTerminalReason,
};
