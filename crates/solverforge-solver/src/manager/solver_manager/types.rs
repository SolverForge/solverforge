use std::error::Error;
use std::fmt::{self, Display};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::super::solution_manager::ScoreAnalysis;
use crate::stats::{CandidateTraceTelemetry, SolverTelemetry};

/// Lifecycle state of a retained solve job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolverLifecycleState {
    /// The worker is solving.
    Solving,
    /// A pause was requested; the worker has not reached a safe point yet.
    PauseRequested,
    /// The worker is paused at a safe point and waits for resume or cancel.
    Paused,
    /// The solve finished and published its final solution.
    Completed,
    /// The solve was cancelled.
    Cancelled,
    /// The solve failed or panicked.
    Failed,
}

impl SolverLifecycleState {
    /// Whether the state is `Completed`, `Cancelled` or `Failed`.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

/// Why a job reached its terminal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolverTerminalReason {
    /// Every phase ran to its natural end.
    Completed,
    /// A configured termination limit stopped the solve.
    TerminatedByConfig,
    /// The job was cancelled.
    Cancelled,
    /// The solve failed or panicked.
    Failed,
}

/// Point-in-time status of a retained job.
#[derive(Debug, Clone, PartialEq)]
pub struct SolverStatus<Sc: Score> {
    /// The job's slot index.
    pub job_id: usize,
    /// Lifecycle state at the time of the query.
    pub lifecycle_state: SolverLifecycleState,
    /// Why the job ended; `None` while it is not terminal.
    pub terminal_reason: Option<SolverTerminalReason>,
    /// Whether the job is paused at a published snapshot that can be resumed
    /// from; cleared when it resumes or ends.
    pub checkpoint_available: bool,
    /// Sequence number of the latest published event; 0 before the first.
    pub event_sequence: u64,
    /// Revision of the newest retained snapshot; `None` before the first.
    pub latest_snapshot_revision: Option<u64>,
    /// Score of the working solution at the latest publication.
    pub current_score: Option<Sc>,
    /// Best score found so far; `None` before the first best solution and
    /// while paused without a snapshot.
    pub best_score: Option<Sc>,
    /// Compact telemetry of the latest publication (without candidate trace).
    pub telemetry: SolverTelemetry,
}

impl<Sc: Score> SolverStatus<Sc> {
    /// Whether the job's lifecycle state is terminal.
    pub fn is_terminal(&self) -> bool {
        self.lifecycle_state.is_terminal()
    }
}

/// One atomically retained diagnostic view of a job.
///
/// Ordinary lifecycle events, status, and snapshots intentionally keep
/// [`SolverTelemetry::candidate_trace`] empty. When a caller explicitly asks
/// for diagnostics, this pairs compact aggregate telemetry with the bounded
/// candidate-pull prefix under one retained-job lock. That prevents combining
/// counters from one publication with a trace from another. Its status scores
/// and `latest_snapshot_revision` are from that same publication, so a caller
/// can fetch and validate exactly the snapshot paired with a terminal trace.
#[derive(Debug, Clone, PartialEq)]
pub struct SolverTelemetryDetail<Sc: Score> {
    /// Status of the publication the trace belongs to.
    pub status: SolverStatus<Sc>,
    /// Candidate-pull trace of that publication; `None` when tracing is off or
    /// a later publication cleared it.
    pub candidate_trace: Option<CandidateTraceTelemetry>,
}

/// Fields shared by every [`SolverEvent`].
#[derive(Debug, Clone, PartialEq)]
pub struct SolverEventMetadata<Sc: Score> {
    /// The job's slot index.
    pub job_id: usize,
    /// Per-job event counter, starting at 1 and increasing by one per event.
    pub event_sequence: u64,
    /// Lifecycle state the event reports.
    pub lifecycle_state: SolverLifecycleState,
    /// Why the job ended; `None` while it is not terminal.
    pub terminal_reason: Option<SolverTerminalReason>,
    /// Compact telemetry at publication (without candidate trace).
    pub telemetry: SolverTelemetry,
    /// Score of the working solution at publication.
    pub current_score: Option<Sc>,
    /// Best score at publication.
    pub best_score: Option<Sc>,
    /// Revision of the snapshot published with this event, or else of the
    /// newest retained snapshot; `None` before the first snapshot.
    pub snapshot_revision: Option<u64>,
}

/// A retained copy of a job's solution with the state it was published in.
#[derive(Debug, Clone)]
pub struct SolverSnapshot<S: PlanningSolution> {
    /// The job's slot index.
    pub job_id: usize,
    /// Per-job snapshot revision, starting at 1.
    pub snapshot_revision: u64,
    /// Lifecycle state at publication.
    pub lifecycle_state: SolverLifecycleState,
    /// Why the job ended, for a terminal snapshot.
    pub terminal_reason: Option<SolverTerminalReason>,
    /// Score of the working solution at publication.
    pub current_score: Option<S::Score>,
    /// Best score at publication.
    pub best_score: Option<S::Score>,
    /// Compact telemetry at publication.
    pub telemetry: SolverTelemetry,
    /// The retained solution.
    pub solution: S,
}

/// Constraint analysis of a retained snapshot.
#[derive(Debug, Clone)]
pub struct SolverSnapshotAnalysis<Sc: Score> {
    /// The job's slot index.
    pub job_id: usize,
    /// Lifecycle state of the analyzed snapshot.
    pub lifecycle_state: SolverLifecycleState,
    /// Terminal reason of the analyzed snapshot.
    pub terminal_reason: Option<SolverTerminalReason>,
    /// Revision of the analyzed snapshot.
    pub snapshot_revision: u64,
    /// The snapshot solution's constraint analysis.
    pub analysis: ScoreAnalysis<Sc>,
}

/// A lifecycle event of a retained solve job, delivered in publication order
/// through a [`SolverEventReceiver`](crate::SolverEventReceiver).
#[derive(Debug, Clone)]
pub enum SolverEvent<S: PlanningSolution> {
    /// Periodic progress with fresh scores and telemetry.
    Progress {
        /// Event metadata.
        metadata: SolverEventMetadata<S::Score>,
    },
    /// A new best solution. Best scores only improve between problem-change
    /// boundaries: after applied real-time changes, the first `BestSolution`
    /// carries the changed problem and may score below earlier ones. Detect a
    /// boundary by `telemetry.problem_changes_applied` increasing.
    BestSolution {
        /// Event metadata; `snapshot_revision` names the snapshot retained for
        /// this solution.
        metadata: SolverEventMetadata<S::Score>,
        /// The new best solution.
        solution: S,
    },
    /// A pause was requested; the worker has not stopped yet.
    PauseRequested {
        /// Event metadata.
        metadata: SolverEventMetadata<S::Score>,
    },
    /// The worker stopped at a safe point.
    Paused {
        /// Event metadata; `snapshot_revision` names the pause snapshot when one
        /// was retained.
        metadata: SolverEventMetadata<S::Score>,
    },
    /// The worker continued after a pause.
    Resumed {
        /// Event metadata.
        metadata: SolverEventMetadata<S::Score>,
    },
    /// The solve finished; terminal.
    Completed {
        /// Event metadata, carrying the terminal reason.
        metadata: SolverEventMetadata<S::Score>,
        /// The final best solution.
        solution: S,
    },
    /// The job was cancelled; terminal.
    Cancelled {
        /// Event metadata.
        metadata: SolverEventMetadata<S::Score>,
    },
    /// The solve failed or panicked; terminal.
    Failed {
        /// Event metadata.
        metadata: SolverEventMetadata<S::Score>,
        /// Failure or panic message.
        error: String,
    },
}

impl<S: PlanningSolution> SolverEvent<S> {
    /// The metadata every variant carries.
    pub fn metadata(&self) -> &SolverEventMetadata<S::Score> {
        match self {
            Self::Progress { metadata }
            | Self::PauseRequested { metadata }
            | Self::Paused { metadata }
            | Self::Resumed { metadata }
            | Self::Cancelled { metadata } => metadata,
            Self::BestSolution { metadata, .. }
            | Self::Completed { metadata, .. }
            | Self::Failed { metadata, .. } => metadata,
        }
    }
}

/// Errors returned by [`SolverManager`](crate::SolverManager) operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverManagerError {
    /// All 16 job slots hold live jobs.
    NoFreeJobSlots,
    /// No live job has this id.
    JobNotFound {
        /// The requested job id.
        job_id: usize,
    },
    /// The job's lifecycle state does not allow the action.
    InvalidStateTransition {
        /// The job id.
        job_id: usize,
        /// The refused action: `pause`, `resume`, `cancel` or `delete`.
        action: &'static str,
        /// The job's state when the action was refused.
        state: SolverLifecycleState,
    },
    /// The job has not retained any snapshot yet.
    NoSnapshotAvailable {
        /// The job id.
        job_id: usize,
    },
    /// The job never retained a snapshot with this revision.
    SnapshotNotFound {
        /// The job id.
        job_id: usize,
        /// The requested revision.
        snapshot_revision: u64,
    },
}

impl Display for SolverManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFreeJobSlots => write!(f, "no free job slots available"),
            Self::JobNotFound { job_id } => write!(f, "job {job_id} was not found"),
            Self::InvalidStateTransition {
                job_id,
                action,
                state,
            } => write!(
                f,
                "cannot {action} job {job_id} while it is in state {state:?}"
            ),
            Self::NoSnapshotAvailable { job_id } => {
                write!(f, "job {job_id} has no retained snapshots")
            }
            Self::SnapshotNotFound {
                job_id,
                snapshot_revision,
            } => write!(
                f,
                "job {job_id} has no retained snapshot revision {snapshot_revision}"
            ),
        }
    }
}

impl Error for SolverManagerError {}
