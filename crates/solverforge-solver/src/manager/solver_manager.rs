/* SolverManager for retained async job lifecycle management.

Provides the high-level API for:
- Starting retained solve jobs that stream lifecycle events
- Tracking authoritative job lifecycle state
- Pausing and resuming jobs at exact runtime-safe boundaries
- Queuing real-time problem changes onto running or paused jobs
- Cancelling and deleting retained jobs
- Retrieving snapshot-bound solutions and score analysis
*/

pub(crate) mod manager;
pub(crate) mod problem_change;
pub(crate) mod receiver;
pub(crate) mod runtime;
mod slot;
pub(crate) mod types;

#[allow(unused_imports)]
pub(crate) use manager::{Solvable, SolverManager, MAX_JOBS};
pub(crate) use runtime::SolverRuntime;
pub(crate) use types::{SolverEvent, SolverLifecycleState, SolverSnapshot, SolverTerminalReason};
