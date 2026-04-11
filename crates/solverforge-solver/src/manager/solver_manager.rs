/* SolverManager for retained async job lifecycle management.

Provides the high-level API for:
- Starting retained solve jobs that stream lifecycle events
- Tracking authoritative job lifecycle state
- Pausing and resuming jobs at exact runtime-safe boundaries
- Cancelling and deleting retained jobs
- Retrieving snapshot-bound solutions and score analysis
*/

use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Condvar, Mutex};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use tokio::sync::mpsc;

use super::solution_manager::{Analyzable, ScoreAnalysis};
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::SolverTelemetry;

/// Maximum concurrent jobs per SolverManager instance.
pub const MAX_JOBS: usize = 16;

const SLOT_FREE: u8 = 0;
const SLOT_SOLVING: u8 = 1;
const SLOT_PAUSE_REQUESTED: u8 = 2;
const SLOT_PAUSED: u8 = 3;
const SLOT_COMPLETED: u8 = 4;
const SLOT_CANCELLED: u8 = 5;
const SLOT_FAILED: u8 = 6;

const SLOT_VISIBLE: u8 = 0;
const SLOT_DELETED: u8 = 1;
const SLOT_DELETING: u8 = 2;

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

/// Runtime context for a retained solve job.
///
/// This is passed into `Solvable::solve()` so the runtime path can publish
/// lifecycle events, settle exact pauses, and observe cancellation.
pub struct SolverRuntime<S: PlanningSolution> {
    job_id: usize,
    slot: &'static JobSlot<S>,
}

impl<S: PlanningSolution> Clone for SolverRuntime<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: PlanningSolution> Copy for SolverRuntime<S> {}

impl<S: PlanningSolution> Debug for SolverRuntime<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SolverRuntime")
            .field("job_id", &self.job_id)
            .finish()
    }
}

impl<S: PlanningSolution> SolverRuntime<S> {
    fn new(job_id: usize, slot: &'static JobSlot<S>) -> Self {
        Self { job_id, slot }
    }

    pub fn job_id(&self) -> usize {
        self.job_id
    }

    pub fn is_cancel_requested(&self) -> bool {
        self.slot.terminate.load(Ordering::Acquire)
    }

    pub(crate) fn is_pause_requested(&self) -> bool {
        self.slot.pause_requested.load(Ordering::Acquire)
    }

    pub(crate) fn cancel_flag(&self) -> &'static AtomicBool {
        &self.slot.terminate
    }

    pub fn emit_progress(
        &self,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
    ) {
        let lifecycle_state = self.current_state();
        self.emit_non_snapshot_event(
            lifecycle_state,
            current_score,
            best_score,
            telemetry,
            EventKind::Progress,
        );
    }

    pub fn emit_best_solution(
        &self,
        solution: S,
        current_score: Option<S::Score>,
        best_score: S::Score,
        telemetry: SolverTelemetry,
    ) {
        let state = self.current_state();
        let (sender, event) = {
            let mut record = self.slot.record.lock().unwrap();
            let terminal_reason = record.terminal_reason;
            record.current_score = current_score;
            record.best_score = Some(best_score);
            record.telemetry = telemetry;

            let snapshot_revision = record.push_snapshot(SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: state,
                terminal_reason,
                current_score,
                best_score: Some(best_score),
                telemetry,
                solution: solution.clone(),
            });

            let metadata = record.next_metadata(self.job_id, state, Some(snapshot_revision));
            let sender = self.slot.sender_clone();
            (sender, SolverEvent::BestSolution { metadata, solution })
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }

    pub fn emit_completed(
        &self,
        solution: S,
        current_score: Option<S::Score>,
        best_score: S::Score,
        telemetry: SolverTelemetry,
        terminal_reason: SolverTerminalReason,
    ) {
        self.slot.state.store(SLOT_COMPLETED, Ordering::SeqCst);
        let (sender, event) = {
            let mut record = self.slot.record.lock().unwrap();
            record.terminal_reason = Some(terminal_reason);
            record.checkpoint_available = false;
            record.current_score = current_score;
            record.best_score = Some(best_score);
            record.telemetry = telemetry;

            let snapshot_revision = record.push_snapshot(SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: SolverLifecycleState::Completed,
                terminal_reason: Some(terminal_reason),
                current_score,
                best_score: Some(best_score),
                telemetry,
                solution: solution.clone(),
            });

            let metadata = record.next_metadata(
                self.job_id,
                SolverLifecycleState::Completed,
                Some(snapshot_revision),
            );
            let sender = self.slot.sender_clone();
            (sender, SolverEvent::Completed { metadata, solution })
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }

    pub fn emit_cancelled(
        &self,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
    ) {
        self.slot.state.store(SLOT_CANCELLED, Ordering::SeqCst);
        self.emit_non_snapshot_terminal_event(
            SolverLifecycleState::Cancelled,
            SolverTerminalReason::Cancelled,
            current_score,
            best_score,
            telemetry,
            EventKind::Cancelled,
        );
    }

    pub fn emit_failed(&self, error: String) {
        if matches!(
            self.current_state(),
            SolverLifecycleState::Completed
                | SolverLifecycleState::Cancelled
                | SolverLifecycleState::Failed
        ) {
            return;
        }

        self.slot.state.store(SLOT_FAILED, Ordering::SeqCst);
        let (sender, event) = {
            let mut record = self.slot.record.lock().unwrap();
            record.terminal_reason = Some(SolverTerminalReason::Failed);
            record.checkpoint_available = false;
            record.failure_message = Some(error.clone());
            let telemetry = record.telemetry;
            let metadata = record.next_metadata(self.job_id, SolverLifecycleState::Failed, None);
            let sender = self.slot.sender_clone();
            (
                sender,
                SolverEvent::Failed {
                    metadata: SolverEventMetadata {
                        telemetry,
                        ..metadata
                    },
                    error,
                },
            )
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }

    pub(crate) fn pause_if_requested<D, ProgressCb>(
        &self,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: solverforge_scoring::Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        if !self.slot.pause_requested.load(Ordering::Acquire) || self.is_cancel_requested() {
            return;
        }

        solver_scope.pause_timers();

        let solution = solver_scope.score_director().clone_working_solution();
        let current_score = solver_scope.current_score().copied();
        let best_score = solver_scope.best_score().copied();
        let telemetry = solver_scope.stats().snapshot();
        let _ = self.pause_with_snapshot(solution, current_score, best_score, telemetry);
        solver_scope.resume_timers();
    }

    pub fn pause_with_snapshot(
        &self,
        solution: S,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
    ) -> bool {
        if !self.slot.pause_requested.load(Ordering::Acquire) || self.is_cancel_requested() {
            return false;
        }

        self.slot.state.store(SLOT_PAUSED, Ordering::SeqCst);
        let (sender, event) = {
            let mut record = self.slot.record.lock().unwrap();
            let terminal_reason = record.terminal_reason;
            record.checkpoint_available = true;
            record.current_score = current_score;
            record.best_score = best_score;
            record.telemetry = telemetry;

            let snapshot_revision = record.push_snapshot(SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: SolverLifecycleState::Paused,
                terminal_reason,
                current_score,
                best_score,
                telemetry,
                solution,
            });

            let metadata = record.next_metadata(
                self.job_id,
                SolverLifecycleState::Paused,
                Some(snapshot_revision),
            );
            let sender = self.slot.sender_clone();
            (sender, SolverEvent::Paused { metadata })
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }

        let mut guard = self.slot.pause_gate.lock().unwrap();
        while self.slot.pause_requested.load(Ordering::Acquire) && !self.is_cancel_requested() {
            guard = self.slot.pause_condvar.wait(guard).unwrap();
        }
        drop(guard);

        if self.is_cancel_requested() {
            return false;
        }

        self.slot.state.store(SLOT_SOLVING, Ordering::SeqCst);
        self.emit_non_snapshot_event(
            SolverLifecycleState::Solving,
            current_score,
            best_score,
            telemetry,
            EventKind::Resumed,
        );
        true
    }

    pub(crate) fn is_terminal(&self) -> bool {
        self.current_state().is_terminal()
    }

    fn current_state(&self) -> SolverLifecycleState {
        self.slot
            .raw_state()
            .expect("runtime accessed a freed job slot")
    }

    fn emit_non_snapshot_event(
        &self,
        lifecycle_state: SolverLifecycleState,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
        kind: EventKind,
    ) {
        let (sender, event) = {
            let mut record = self.slot.record.lock().unwrap();
            record.current_score = current_score;
            record.best_score = best_score;
            record.telemetry = telemetry;
            if lifecycle_state != SolverLifecycleState::Paused {
                record.checkpoint_available = false;
            }
            let metadata = record.next_metadata(self.job_id, lifecycle_state, None);
            let sender = self.slot.sender_clone();
            let event = match kind {
                EventKind::Progress => SolverEvent::Progress { metadata },
                EventKind::Resumed => SolverEvent::Resumed { metadata },
                EventKind::Cancelled => unreachable!(),
            };
            (sender, event)
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }

    fn emit_non_snapshot_terminal_event(
        &self,
        lifecycle_state: SolverLifecycleState,
        terminal_reason: SolverTerminalReason,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
        kind: EventKind,
    ) {
        let (sender, event) = {
            let mut record = self.slot.record.lock().unwrap();
            record.terminal_reason = Some(terminal_reason);
            record.checkpoint_available = false;
            record.current_score = current_score;
            record.best_score = best_score;
            record.telemetry = telemetry;
            let metadata = record.next_metadata(self.job_id, lifecycle_state, None);
            let sender = self.slot.sender_clone();
            let event = match kind {
                EventKind::Cancelled => SolverEvent::Cancelled { metadata },
                EventKind::Progress | EventKind::Resumed => unreachable!(),
            };
            (sender, event)
        };

        if let Some(sender) = sender {
            let _ = sender.send(event);
        }
    }
}

/// Trait for solutions that can run inside the retained lifecycle manager.
pub trait Solvable: PlanningSolution + Send + 'static {
    fn solve(self, runtime: SolverRuntime<Self>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    Progress,
    Resumed,
    Cancelled,
}

struct JobRecord<S: PlanningSolution> {
    terminal_reason: Option<SolverTerminalReason>,
    event_sequence: u64,
    latest_snapshot_revision: Option<u64>,
    current_score: Option<S::Score>,
    best_score: Option<S::Score>,
    telemetry: SolverTelemetry,
    checkpoint_available: bool,
    snapshots: Vec<SolverSnapshot<S>>,
    failure_message: Option<String>,
}

impl<S: PlanningSolution> JobRecord<S> {
    const fn new() -> Self {
        Self {
            terminal_reason: None,
            event_sequence: 0,
            latest_snapshot_revision: None,
            current_score: None,
            best_score: None,
            telemetry: SolverTelemetry {
                elapsed_ms: 0,
                step_count: 0,
                moves_evaluated: 0,
                moves_accepted: 0,
                score_calculations: 0,
                moves_per_second: 0,
                acceptance_rate: 0.0,
            },
            checkpoint_available: false,
            snapshots: Vec::new(),
            failure_message: None,
        }
    }

    fn reset(&mut self) {
        self.terminal_reason = None;
        self.event_sequence = 0;
        self.latest_snapshot_revision = None;
        self.current_score = None;
        self.best_score = None;
        self.telemetry = SolverTelemetry {
            elapsed_ms: 0,
            step_count: 0,
            moves_evaluated: 0,
            moves_accepted: 0,
            score_calculations: 0,
            moves_per_second: 0,
            acceptance_rate: 0.0,
        };
        self.checkpoint_available = false;
        self.snapshots.clear();
        self.failure_message = None;
    }

    fn push_snapshot(&mut self, mut snapshot: SolverSnapshot<S>) -> u64 {
        let next = self.latest_snapshot_revision.unwrap_or(0) + 1;
        snapshot.snapshot_revision = next;
        self.latest_snapshot_revision = Some(next);
        self.snapshots.push(snapshot);
        next
    }

    fn next_metadata(
        &mut self,
        job_id: usize,
        lifecycle_state: SolverLifecycleState,
        snapshot_revision: Option<u64>,
    ) -> SolverEventMetadata<S::Score> {
        self.event_sequence += 1;
        SolverEventMetadata {
            job_id,
            event_sequence: self.event_sequence,
            lifecycle_state,
            terminal_reason: self.terminal_reason,
            telemetry: self.telemetry,
            current_score: self.current_score,
            best_score: self.best_score,
            snapshot_revision: snapshot_revision.or(self.latest_snapshot_revision),
        }
    }

    fn status(
        &self,
        job_id: usize,
        lifecycle_state: SolverLifecycleState,
    ) -> SolverStatus<S::Score> {
        SolverStatus {
            job_id,
            lifecycle_state,
            terminal_reason: self.terminal_reason,
            checkpoint_available: self.checkpoint_available,
            event_sequence: self.event_sequence,
            latest_snapshot_revision: self.latest_snapshot_revision,
            current_score: self.current_score,
            best_score: self.best_score,
            telemetry: self.telemetry,
        }
    }
}

struct JobSlot<S: PlanningSolution> {
    state: AtomicU8,
    visibility: AtomicU8,
    terminate: AtomicBool,
    pause_requested: AtomicBool,
    worker_running: AtomicBool,
    sender: Mutex<Option<mpsc::UnboundedSender<SolverEvent<S>>>>,
    record: Mutex<JobRecord<S>>,
    pause_gate: Mutex<()>,
    pause_condvar: Condvar,
}

impl<S: PlanningSolution> JobSlot<S> {
    const fn new() -> Self {
        Self {
            state: AtomicU8::new(SLOT_FREE),
            visibility: AtomicU8::new(SLOT_VISIBLE),
            terminate: AtomicBool::new(false),
            pause_requested: AtomicBool::new(false),
            worker_running: AtomicBool::new(false),
            sender: Mutex::new(None),
            record: Mutex::new(JobRecord::new()),
            pause_gate: Mutex::new(()),
            pause_condvar: Condvar::new(),
        }
    }

    fn sender_clone(&self) -> Option<mpsc::UnboundedSender<SolverEvent<S>>> {
        self.sender.lock().unwrap().clone()
    }

    fn initialize(&self, sender: mpsc::UnboundedSender<SolverEvent<S>>) {
        self.terminate.store(false, Ordering::Release);
        self.pause_requested.store(false, Ordering::Release);
        self.worker_running.store(true, Ordering::Release);
        self.visibility.store(SLOT_VISIBLE, Ordering::Release);
        *self.sender.lock().unwrap() = Some(sender);
        self.record.lock().unwrap().reset();
    }

    fn reset(&self) {
        self.terminate.store(false, Ordering::Release);
        self.pause_requested.store(false, Ordering::Release);
        self.worker_running.store(false, Ordering::Release);
        *self.sender.lock().unwrap() = None;
        self.record.lock().unwrap().reset();
        self.state.store(SLOT_FREE, Ordering::Release);
        self.visibility.store(SLOT_VISIBLE, Ordering::Release);
    }

    fn mark_deleted(&self) {
        self.visibility.store(SLOT_DELETED, Ordering::Release);
        *self.sender.lock().unwrap() = None;
    }

    fn worker_exited(&self) {
        self.worker_running.store(false, Ordering::Release);
        self.try_reset_deleted();
    }

    fn try_reset_deleted(&self) {
        if self.worker_running.load(Ordering::Acquire) {
            return;
        }

        if self
            .visibility
            .compare_exchange(
                SLOT_DELETED,
                SLOT_DELETING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            self.reset();
        }
    }

    fn raw_state(&self) -> Option<SolverLifecycleState> {
        match self.state.load(Ordering::Acquire) {
            SLOT_SOLVING => Some(SolverLifecycleState::Solving),
            SLOT_PAUSE_REQUESTED => Some(SolverLifecycleState::PauseRequested),
            SLOT_PAUSED => Some(SolverLifecycleState::Paused),
            SLOT_COMPLETED => Some(SolverLifecycleState::Completed),
            SLOT_CANCELLED => Some(SolverLifecycleState::Cancelled),
            SLOT_FAILED => Some(SolverLifecycleState::Failed),
            _ => None,
        }
    }

    fn public_state(&self) -> Option<SolverLifecycleState> {
        if self.visibility.load(Ordering::Acquire) != SLOT_VISIBLE {
            return None;
        }

        self.raw_state()
    }
}

/// Manages retained async solve jobs with lifecycle-complete event streaming.
pub struct SolverManager<S: Solvable> {
    slots: [JobSlot<S>; MAX_JOBS],
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S: Solvable> Default for SolverManager<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Solvable> SolverManager<S>
where
    S::Score: Score,
{
    pub const fn new() -> Self {
        Self {
            slots: [
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
                JobSlot::new(),
            ],
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn solve(
        &'static self,
        solution: S,
    ) -> Result<(usize, mpsc::UnboundedReceiver<SolverEvent<S>>), SolverManagerError> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let Some(slot_idx) = self.slots.iter().position(|slot| {
            slot.state
                .compare_exchange(SLOT_FREE, SLOT_SOLVING, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
        }) else {
            return Err(SolverManagerError::NoFreeJobSlots);
        };

        let slot = &self.slots[slot_idx];
        slot.initialize(sender);
        let runtime = SolverRuntime::new(slot_idx, slot);

        rayon::spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| solution.solve(runtime)));

            match result {
                Ok(()) => {
                    if !runtime.is_terminal() {
                        if runtime.is_cancel_requested() {
                            let (current_score, best_score, telemetry) = {
                                let record = runtime.slot.record.lock().unwrap();
                                (record.current_score, record.best_score, record.telemetry)
                            };
                            runtime.emit_cancelled(current_score, best_score, telemetry);
                        } else {
                            runtime.emit_failed(
                                "solver returned without emitting a terminal lifecycle event"
                                    .to_string(),
                            );
                        }
                    }
                }
                Err(payload) => {
                    runtime.emit_failed(panic_payload_to_string(payload));
                }
            }

            runtime.slot.worker_exited();
        });

        Ok((slot_idx, receiver))
    }

    pub fn get_status(&self, job_id: usize) -> Result<SolverStatus<S::Score>, SolverManagerError> {
        let slot = self.slot(job_id)?;
        let state = slot
            .public_state()
            .ok_or(SolverManagerError::JobNotFound { job_id })?;
        let record = slot.record.lock().unwrap();
        Ok(record.status(job_id, state))
    }

    pub fn pause(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        match slot.state.compare_exchange(
            SLOT_SOLVING,
            SLOT_PAUSE_REQUESTED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                slot.pause_requested.store(true, Ordering::SeqCst);
                let (sender, event) = {
                    let mut record = slot.record.lock().unwrap();
                    let metadata =
                        record.next_metadata(job_id, SolverLifecycleState::PauseRequested, None);
                    let sender = slot.sender_clone();
                    (sender, SolverEvent::PauseRequested { metadata })
                };
                if let Some(sender) = sender {
                    let _ = sender.send(event);
                }
                Ok(())
            }
            Err(_) => {
                let state = slot
                    .public_state()
                    .ok_or(SolverManagerError::JobNotFound { job_id })?;
                Err(SolverManagerError::InvalidStateTransition {
                    job_id,
                    action: "pause",
                    state,
                })
            }
        }
    }

    pub fn resume(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        let state = slot
            .public_state()
            .ok_or(SolverManagerError::JobNotFound { job_id })?;
        if state != SolverLifecycleState::Paused {
            return Err(SolverManagerError::InvalidStateTransition {
                job_id,
                action: "resume",
                state,
            });
        }

        slot.pause_requested.store(false, Ordering::SeqCst);
        slot.pause_condvar.notify_one();
        Ok(())
    }

    pub fn cancel(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        let state = slot
            .public_state()
            .ok_or(SolverManagerError::JobNotFound { job_id })?;
        if !matches!(
            state,
            SolverLifecycleState::Solving
                | SolverLifecycleState::PauseRequested
                | SolverLifecycleState::Paused
        ) {
            return Err(SolverManagerError::InvalidStateTransition {
                job_id,
                action: "cancel",
                state,
            });
        }

        slot.terminate.store(true, Ordering::SeqCst);
        slot.pause_requested.store(false, Ordering::SeqCst);
        slot.pause_condvar.notify_one();
        Ok(())
    }

    pub fn delete(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        let state = slot
            .public_state()
            .ok_or(SolverManagerError::JobNotFound { job_id })?;
        if !state.is_terminal() {
            return Err(SolverManagerError::InvalidStateTransition {
                job_id,
                action: "delete",
                state,
            });
        }

        slot.mark_deleted();
        slot.try_reset_deleted();
        Ok(())
    }

    pub fn get_snapshot(
        &self,
        job_id: usize,
        snapshot_revision: Option<u64>,
    ) -> Result<SolverSnapshot<S>, SolverManagerError> {
        let slot = self.slot(job_id)?;
        if slot.public_state().is_none() {
            return Err(SolverManagerError::JobNotFound { job_id });
        }

        let record = slot.record.lock().unwrap();
        if record.snapshots.is_empty() {
            return Err(SolverManagerError::NoSnapshotAvailable { job_id });
        }

        match snapshot_revision {
            Some(revision) => record
                .snapshots
                .iter()
                .find(|snapshot| snapshot.snapshot_revision == revision)
                .cloned()
                .ok_or(SolverManagerError::SnapshotNotFound {
                    job_id,
                    snapshot_revision: revision,
                }),
            None => Ok(record
                .snapshots
                .last()
                .expect("checked non-empty snapshots")
                .clone()),
        }
    }

    pub fn analyze_snapshot(
        &self,
        job_id: usize,
        snapshot_revision: Option<u64>,
    ) -> Result<SolverSnapshotAnalysis<S::Score>, SolverManagerError>
    where
        S: Analyzable,
    {
        let snapshot = self.get_snapshot(job_id, snapshot_revision)?;
        Ok(SolverSnapshotAnalysis {
            job_id,
            lifecycle_state: snapshot.lifecycle_state,
            terminal_reason: snapshot.terminal_reason,
            snapshot_revision: snapshot.snapshot_revision,
            analysis: snapshot.solution.analyze(),
        })
    }

    pub fn active_job_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.public_state().is_some())
            .count()
    }

    #[cfg(test)]
    pub(crate) fn slot_is_free_for_test(&self, job_id: usize) -> bool {
        self.slots
            .get(job_id)
            .is_some_and(|slot| slot.state.load(Ordering::Acquire) == SLOT_FREE)
    }

    fn slot(&self, job_id: usize) -> Result<&JobSlot<S>, SolverManagerError> {
        self.slots
            .get(job_id)
            .ok_or(SolverManagerError::JobNotFound { job_id })
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "solver panicked with a non-string payload".to_string()
    }
}
