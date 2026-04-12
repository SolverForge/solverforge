use std::error::Error;
use std::fmt::{self, Display};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::super::solution_manager::ScoreAnalysis;
use crate::stats::SolverTelemetry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SolverLifecycleState {
    Solving,
    PauseRequested,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl SolverLifecycleState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SolverTerminalReason {
    Completed,
    TerminatedByConfig,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolverStatus<Sc: Score> {
    pub job_id: usize,
    pub lifecycle_state: SolverLifecycleState,
    pub terminal_reason: Option<SolverTerminalReason>,
    pub checkpoint_available: bool,
    pub event_sequence: u64,
    pub latest_snapshot_revision: Option<u64>,
    pub current_score: Option<Sc>,
    pub best_score: Option<Sc>,
    pub telemetry: SolverTelemetry,
}

impl<Sc: Score> SolverStatus<Sc> {
    pub fn is_terminal(&self) -> bool {
        self.lifecycle_state.is_terminal()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolverEventMetadata<Sc: Score> {
    pub job_id: usize,
    pub event_sequence: u64,
    pub lifecycle_state: SolverLifecycleState,
    pub terminal_reason: Option<SolverTerminalReason>,
    pub telemetry: SolverTelemetry,
    pub current_score: Option<Sc>,
    pub best_score: Option<Sc>,
    pub snapshot_revision: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct SolverSnapshot<S: PlanningSolution> {
    pub job_id: usize,
    pub snapshot_revision: u64,
    pub lifecycle_state: SolverLifecycleState,
    pub terminal_reason: Option<SolverTerminalReason>,
    pub current_score: Option<S::Score>,
    pub best_score: Option<S::Score>,
    pub telemetry: SolverTelemetry,
    pub solution: S,
}

#[derive(Debug, Clone)]
pub struct SolverSnapshotAnalysis<Sc: Score> {
    pub job_id: usize,
    pub lifecycle_state: SolverLifecycleState,
    pub terminal_reason: Option<SolverTerminalReason>,
    pub snapshot_revision: u64,
    pub analysis: ScoreAnalysis<Sc>,
}

#[derive(Debug, Clone)]
pub enum SolverEvent<S: PlanningSolution> {
    Progress {
        metadata: SolverEventMetadata<S::Score>,
    },
    BestSolution {
        metadata: SolverEventMetadata<S::Score>,
        solution: S,
    },
    PauseRequested {
        metadata: SolverEventMetadata<S::Score>,
    },
    Paused {
        metadata: SolverEventMetadata<S::Score>,
    },
    Resumed {
        metadata: SolverEventMetadata<S::Score>,
    },
    Completed {
        metadata: SolverEventMetadata<S::Score>,
        solution: S,
    },
    Cancelled {
        metadata: SolverEventMetadata<S::Score>,
    },
    Failed {
        metadata: SolverEventMetadata<S::Score>,
        error: String,
    },
}

impl<S: PlanningSolution> SolverEvent<S> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverManagerError {
    NoFreeJobSlots,
    JobNotFound {
        job_id: usize,
    },
    InvalidStateTransition {
        job_id: usize,
        action: &'static str,
        state: SolverLifecycleState,
    },
    NoSnapshotAvailable {
        job_id: usize,
    },
    SnapshotNotFound {
        job_id: usize,
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
