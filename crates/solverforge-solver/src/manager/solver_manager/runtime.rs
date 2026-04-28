use std::fmt::{self, Debug};
use std::sync::atomic::{AtomicBool, Ordering};

use solverforge_core::domain::PlanningSolution;

use super::slot::{
    JobSlot, SLOT_CANCELLED, SLOT_COMPLETED, SLOT_FAILED, SLOT_PAUSED, SLOT_SOLVING,
};
use super::types::{SolverEvent, SolverEventMetadata, SolverLifecycleState, SolverTerminalReason};
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::SolverTelemetry;

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

impl<S: PlanningSolution> SolverRuntime<S> {
    pub(super) fn new(job_id: usize, slot: &'static JobSlot<S>) -> Self {
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
        self.slot.with_publication(|sender, record| {
            let terminal_reason = record.terminal_reason;
            record.current_score = current_score;
            record.best_score = Some(best_score);
            record.telemetry = telemetry.clone();

            let snapshot_revision = record.push_snapshot(super::types::SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: state,
                terminal_reason,
                current_score,
                best_score: Some(best_score),
                telemetry: telemetry.clone(),
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
        self.slot.with_publication(|sender, record| {
            self.slot.state.store(SLOT_COMPLETED, Ordering::SeqCst);
            record.terminal_reason = Some(terminal_reason);
            record.checkpoint_available = false;
            record.current_score = current_score;
            record.best_score = Some(best_score);
            record.telemetry = telemetry.clone();

            let snapshot_revision = record.push_snapshot(super::types::SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: SolverLifecycleState::Completed,
                terminal_reason: Some(terminal_reason),
                current_score,
                best_score: Some(best_score),
                telemetry: telemetry.clone(),
                solution: solution.clone(),
            });

            let metadata = record.next_metadata(
                self.job_id,
                SolverLifecycleState::Completed,
                Some(snapshot_revision),
            );
            if let Some(sender) = sender {
                let _ = sender.send(SolverEvent::Completed { metadata, solution });
            }
        });
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
        if matches!(
            self.current_state(),
            SolverLifecycleState::Completed
                | SolverLifecycleState::Cancelled
                | SolverLifecycleState::Failed
        ) {
            return;
        }

        self.slot.with_publication(|sender, record| {
            self.slot.state.store(SLOT_FAILED, Ordering::SeqCst);
            record.terminal_reason = Some(SolverTerminalReason::Failed);
            record.checkpoint_available = false;
            record.failure_message = Some(error.clone());
            let telemetry = record.telemetry.clone();
            let metadata = record.next_metadata(self.job_id, SolverLifecycleState::Failed, None);
            if let Some(sender) = sender {
                let _ = sender.send(SolverEvent::Failed {
                    metadata: SolverEventMetadata {
                        telemetry,
                        ..metadata
                    },
                    error,
                });
            }
        });
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

        self.slot.with_publication(|sender, record| {
            self.slot.state.store(SLOT_PAUSED, Ordering::SeqCst);
            let terminal_reason = record.terminal_reason;
            record.checkpoint_available = true;
            record.current_score = current_score;
            record.best_score = best_score;
            record.telemetry = telemetry.clone();

            let snapshot_revision = record.push_snapshot(super::types::SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: SolverLifecycleState::Paused,
                terminal_reason,
                current_score,
                best_score,
                telemetry: telemetry.clone(),
                solution,
            });

            let metadata = record.next_metadata(
                self.job_id,
                SolverLifecycleState::Paused,
                Some(snapshot_revision),
            );
            if let Some(sender) = sender {
                let _ = sender.send(SolverEvent::Paused { metadata });
            }
        });

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
        self.slot.with_publication(|sender, record| {
            record.current_score = current_score;
            record.best_score = best_score;
            record.telemetry = telemetry.clone();
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
            record.telemetry = telemetry.clone();
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

pub(super) fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "solver panicked with a non-string payload".to_string()
    }
}
