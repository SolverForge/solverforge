use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Condvar, Mutex};

use solverforge_core::domain::PlanningSolution;
use tokio::sync::mpsc;

use super::types::{
    SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverSnapshot, SolverStatus,
    SolverTerminalReason,
};
use crate::stats::SolverTelemetry;

pub(super) const SLOT_FREE: u8 = 0;
pub(super) const SLOT_SOLVING: u8 = 1;
pub(super) const SLOT_PAUSE_REQUESTED: u8 = 2;
pub(super) const SLOT_PAUSED: u8 = 3;
pub(super) const SLOT_COMPLETED: u8 = 4;
pub(super) const SLOT_CANCELLED: u8 = 5;
pub(super) const SLOT_FAILED: u8 = 6;

const SLOT_VISIBLE: u8 = 0;
const SLOT_DELETED: u8 = 1;
const SLOT_DELETING: u8 = 2;

pub(super) struct JobRecord<S: PlanningSolution> {
    pub(super) terminal_reason: Option<SolverTerminalReason>,
    pub(super) event_sequence: u64,
    pub(super) latest_snapshot_revision: Option<u64>,
    pub(super) current_score: Option<S::Score>,
    pub(super) best_score: Option<S::Score>,
    pub(super) telemetry: SolverTelemetry,
    pub(super) checkpoint_available: bool,
    pub(super) snapshots: Vec<SolverSnapshot<S>>,
    pub(super) failure_message: Option<String>,
}

impl<S: PlanningSolution> JobRecord<S> {
    pub(super) const fn new() -> Self {
        Self {
            terminal_reason: None,
            event_sequence: 0,
            latest_snapshot_revision: None,
            current_score: None,
            best_score: None,
            telemetry: SolverTelemetry::new_const(),
            checkpoint_available: false,
            snapshots: Vec::new(),
            failure_message: None,
        }
    }

    pub(super) fn reset(&mut self) {
        self.terminal_reason = None;
        self.event_sequence = 0;
        self.latest_snapshot_revision = None;
        self.current_score = None;
        self.best_score = None;
        self.telemetry = SolverTelemetry::default();
        self.checkpoint_available = false;
        self.snapshots.clear();
        self.failure_message = None;
    }

    pub(super) fn push_snapshot(&mut self, mut snapshot: SolverSnapshot<S>) -> u64 {
        let next = self.latest_snapshot_revision.unwrap_or(0) + 1;
        snapshot.snapshot_revision = next;
        self.latest_snapshot_revision = Some(next);
        self.snapshots.push(snapshot);
        next
    }

    pub(super) fn next_metadata(
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
            telemetry: self.telemetry.clone(),
            current_score: self.current_score,
            best_score: self.best_score,
            snapshot_revision: snapshot_revision.or(self.latest_snapshot_revision),
        }
    }

    pub(super) fn status(
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
            telemetry: self.telemetry.clone(),
        }
    }
}

pub(super) struct JobSlot<S: PlanningSolution> {
    pub(super) state: AtomicU8,
    pub(super) visibility: AtomicU8,
    pub(super) terminate: AtomicBool,
    pub(super) pause_requested: AtomicBool,
    pub(super) worker_running: AtomicBool,
    publication: Mutex<()>,
    sender: Mutex<Option<mpsc::UnboundedSender<SolverEvent<S>>>>,
    pub(super) record: Mutex<JobRecord<S>>,
    pub(super) pause_gate: Mutex<()>,
    pub(super) pause_condvar: Condvar,
}

impl<S: PlanningSolution> JobSlot<S> {
    pub(super) const fn new() -> Self {
        Self {
            state: AtomicU8::new(SLOT_FREE),
            visibility: AtomicU8::new(SLOT_VISIBLE),
            terminate: AtomicBool::new(false),
            pause_requested: AtomicBool::new(false),
            worker_running: AtomicBool::new(false),
            publication: Mutex::new(()),
            sender: Mutex::new(None),
            record: Mutex::new(JobRecord::new()),
            pause_gate: Mutex::new(()),
            pause_condvar: Condvar::new(),
        }
    }

    fn sender_clone(&self) -> Option<mpsc::UnboundedSender<SolverEvent<S>>> {
        self.sender.lock().unwrap().clone()
    }

    pub(super) fn with_publication<R>(
        &self,
        f: impl FnOnce(Option<mpsc::UnboundedSender<SolverEvent<S>>>, &mut JobRecord<S>) -> R,
    ) -> R {
        let _publication = self.publication.lock().unwrap();
        let sender = self.sender_clone();
        let mut record = self.record.lock().unwrap();
        f(sender, &mut record)
    }

    pub(super) fn initialize(&self, sender: mpsc::UnboundedSender<SolverEvent<S>>) {
        self.terminate.store(false, Ordering::Release);
        self.pause_requested.store(false, Ordering::Release);
        self.worker_running.store(true, Ordering::Release);
        self.visibility.store(SLOT_VISIBLE, Ordering::Release);
        *self.sender.lock().unwrap() = Some(sender);
        self.record.lock().unwrap().reset();
    }

    pub(super) fn reset(&self) {
        self.terminate.store(false, Ordering::Release);
        self.pause_requested.store(false, Ordering::Release);
        self.worker_running.store(false, Ordering::Release);
        *self.sender.lock().unwrap() = None;
        self.record.lock().unwrap().reset();
        self.state.store(SLOT_FREE, Ordering::Release);
        self.visibility.store(SLOT_VISIBLE, Ordering::Release);
    }

    pub(super) fn mark_deleted(&self) {
        self.visibility.store(SLOT_DELETED, Ordering::Release);
        *self.sender.lock().unwrap() = None;
    }

    pub(super) fn worker_exited(&self) {
        self.worker_running.store(false, Ordering::Release);
        self.try_reset_deleted();
    }

    pub(super) fn try_reset_deleted(&self) {
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

    pub(super) fn raw_state(&self) -> Option<SolverLifecycleState> {
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

    pub(super) fn public_state(&self) -> Option<SolverLifecycleState> {
        if self.visibility.load(Ordering::Acquire) != SLOT_VISIBLE {
            return None;
        }

        self.raw_state()
    }
}
