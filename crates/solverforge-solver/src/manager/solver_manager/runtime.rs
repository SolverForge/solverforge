use std::any::Any;
use std::fmt::{self, Debug};
use std::sync::atomic::{AtomicBool, Ordering};

use solverforge_core::domain::PlanningSolution;

use super::slot::{JobSlot, SLOT_CANCELLED, SLOT_COMPLETED, SLOT_FAILED, SLOT_SOLVING};
use super::types::{SolverEvent, SolverLifecycleState, SolverTerminalReason};
use crate::stats::SolverTelemetry;

mod pause;

/// Preserves a foreign-runtime error object while giving retained solving a
/// stable, displayable failure message.
pub struct SolverPanicPayload {
    message: String,
    payload: Box<dyn Any + Send>,
}

impl SolverPanicPayload {
    pub fn new(message: impl Into<String>, payload: impl Any + Send) -> Self {
        Self {
            message: message.into(),
            payload: Box::new(payload),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn into_parts(self) -> (String, Box<dyn Any + Send>) {
        (self.message, self.payload)
    }
}

impl Debug for SolverPanicPayload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SolverPanicPayload")
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}

/// Runtime context for a retained solve job.
///
/// This is passed into `Solvable::solve()` so the runtime path can publish
/// lifecycle events, settle exact pauses, and observe cancellation.
pub struct SolverRuntime<S: PlanningSolution> {
    job_id: usize,
    pub(super) slot: &'static JobSlot<S>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    Progress,
    Resumed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionDisposition {
    Published,
    Pause,
    Cancel,
    AlreadyTerminal,
}

impl<S: PlanningSolution> SolverRuntime<S> {
    pub(super) fn new(job_id: usize, slot: &'static JobSlot<S>) -> Self {
        Self { job_id, slot }
    }

    /// Creates a runtime handle for synchronous solves that are not retained by
    /// a [`SolverManager`](super::SolverManager).
    ///
    /// Detached runtimes publish lifecycle state into an internal slot without
    /// an event receiver. Retained solves should continue to use
    /// `SolverManager`, which owns reusable slots and event delivery.
    pub fn detached() -> Self {
        let slot = Box::leak(Box::new(JobSlot::new()));
        slot.state.store(SLOT_SOLVING, Ordering::Release);
        slot.worker_running.store(true, Ordering::Release);
        Self {
            job_id: usize::MAX,
            slot,
        }
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
        self.emit_non_snapshot_event(current_score, best_score, telemetry, EventKind::Progress);
    }

    pub fn emit_best_solution(
        &self,
        solution: S,
        current_score: Option<S::Score>,
        best_score: S::Score,
        telemetry: SolverTelemetry,
    ) {
        self.slot.with_publication(|sender, record| {
            let state = self.current_state();
            let terminal_reason = record.terminal_reason;
            record.current_score = current_score;
            record.best_score = Some(best_score);
            record.publish_telemetry(telemetry);

            let snapshot_revision = record.push_snapshot(super::types::SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: state,
                terminal_reason,
                current_score,
                best_score: Some(best_score),
                telemetry: record.telemetry.clone(),
                solution: solution.clone(),
            });

            let metadata = record.next_metadata(self.job_id, state, Some(snapshot_revision));
            if let Some(sender) = sender {
                let _ = sender.send(SolverEvent::BestSolution { metadata, solution });
            }
        });
    }

    pub fn emit_completed(
        &self,
        solution: S,
        current_score: Option<S::Score>,
        best_score: S::Score,
        telemetry: SolverTelemetry,
        terminal_reason: SolverTerminalReason,
    ) {
        loop {
            let disposition = self.slot.with_publication(|sender, record| {
                if self.current_state().is_terminal() {
                    return CompletionDisposition::AlreadyTerminal;
                }
                if self.is_cancel_requested() {
                    return CompletionDisposition::Cancel;
                }
                if self.is_pause_requested()
                    || matches!(
                        self.current_state(),
                        SolverLifecycleState::PauseRequested | SolverLifecycleState::Paused
                    )
                {
                    return CompletionDisposition::Pause;
                }

                self.slot.state.store(SLOT_COMPLETED, Ordering::SeqCst);
                record.terminal_reason = Some(terminal_reason);
                record.checkpoint_available = false;
                record.current_score = current_score;
                record.best_score = Some(best_score);
                record.publish_telemetry(telemetry.clone());

                let snapshot_revision = record.push_snapshot(super::types::SolverSnapshot {
                    job_id: self.job_id,
                    snapshot_revision: 0,
                    lifecycle_state: SolverLifecycleState::Completed,
                    terminal_reason: Some(terminal_reason),
                    current_score,
                    best_score: Some(best_score),
                    telemetry: record.telemetry.clone(),
                    solution: solution.clone(),
                });

                let metadata = record.next_metadata(
                    self.job_id,
                    SolverLifecycleState::Completed,
                    Some(snapshot_revision),
                );
                if let Some(sender) = sender {
                    let _ = sender.send(SolverEvent::Completed {
                        metadata,
                        solution: solution.clone(),
                    });
                }
                CompletionDisposition::Published
            });

            match disposition {
                CompletionDisposition::Published | CompletionDisposition::AlreadyTerminal => {
                    return;
                }
                CompletionDisposition::Cancel => {
                    self.emit_cancelled(current_score, Some(best_score), telemetry);
                    return;
                }
                CompletionDisposition::Pause => {
                    if self.pause_with_snapshot(
                        solution.clone(),
                        current_score,
                        Some(best_score),
                        telemetry.clone(),
                    ) {
                        continue;
                    }
                    if self.is_cancel_requested() {
                        self.emit_cancelled(current_score, Some(best_score), telemetry);
                        return;
                    }
                }
            }
        }
    }

    pub fn emit_cancelled(
        &self,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
    ) {
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
        self.slot.with_publication(|sender, record| {
            if matches!(
                self.current_state(),
                SolverLifecycleState::Completed
                    | SolverLifecycleState::Cancelled
                    | SolverLifecycleState::Failed
            ) {
                return;
            }
            self.slot.state.store(SLOT_FAILED, Ordering::SeqCst);
            record.terminal_reason = Some(SolverTerminalReason::Failed);
            record.checkpoint_available = false;
            record.failure_message = Some(error.clone());
            record.clear_candidate_trace_detail();
            let metadata = record.next_metadata(self.job_id, SolverLifecycleState::Failed, None);
            if let Some(sender) = sender {
                let _ = sender.send(SolverEvent::Failed { metadata, error });
            }
        });
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
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
        kind: EventKind,
    ) {
        self.slot.with_publication(|sender, record| {
            if kind == EventKind::Resumed && self.is_cancel_requested() {
                return;
            }
            let lifecycle_state = match kind {
                EventKind::Progress => self.current_state(),
                EventKind::Resumed => {
                    self.slot.state.store(SLOT_SOLVING, Ordering::SeqCst);
                    SolverLifecycleState::Solving
                }
                EventKind::Cancelled => unreachable!(),
            };
            record.current_score = current_score;
            record.best_score = best_score;
            record.publish_telemetry(telemetry);
            if lifecycle_state != SolverLifecycleState::Paused {
                record.checkpoint_available = false;
            }
            let metadata = record.next_metadata(self.job_id, lifecycle_state, None);
            let event = match kind {
                EventKind::Progress => SolverEvent::Progress { metadata },
                EventKind::Resumed => SolverEvent::Resumed { metadata },
                EventKind::Cancelled => unreachable!(),
            };
            if let Some(sender) = sender {
                let _ = sender.send(event);
            }
        });
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
        self.slot.with_publication(|sender, record| {
            if self.current_state().is_terminal() {
                return;
            }
            match lifecycle_state {
                SolverLifecycleState::Cancelled => {
                    self.slot.state.store(SLOT_CANCELLED, Ordering::SeqCst);
                }
                SolverLifecycleState::Failed => {
                    self.slot.state.store(SLOT_FAILED, Ordering::SeqCst);
                }
                _ => {}
            }
            record.terminal_reason = Some(terminal_reason);
            record.checkpoint_available = false;
            record.current_score = current_score;
            record.best_score = best_score;
            record.publish_telemetry(telemetry);
            let metadata = record.next_metadata(self.job_id, lifecycle_state, None);
            let event = match kind {
                EventKind::Cancelled => SolverEvent::Cancelled { metadata },
                EventKind::Progress | EventKind::Resumed => unreachable!(),
            };
            if let Some(sender) = sender {
                let _ = sender.send(event);
            }
        });
    }
}

pub(super) fn panic_payload_to_string(payload: Box<dyn Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(payload) = payload.downcast_ref::<SolverPanicPayload>() {
        payload.message().to_string()
    } else {
        "solver panicked with a non-string payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{panic_payload_to_string, SolverPanicPayload};

    #[test]
    fn retained_failure_uses_foreign_runtime_message() {
        let payload = SolverPanicPayload::new("foreign traceback", 41_u8);
        assert_eq!(
            panic_payload_to_string(Box::new(payload)),
            "foreign traceback"
        );
    }

    #[test]
    fn foreign_runtime_payload_remains_owned() {
        let payload = SolverPanicPayload::new("foreign traceback", 41_u8);
        let (message, payload) = payload.into_parts();
        assert_eq!(message, "foreign traceback");
        assert_eq!(*payload.downcast::<u8>().expect("u8 payload"), 41);
    }
}
