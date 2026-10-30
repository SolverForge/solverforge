use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::Ordering;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use tokio::sync::mpsc;

use crate::stats::QualifiedCandidateTraceRunProvenance;
use crate::RuntimeBuildResult;

use super::super::solution_manager::Analyzable;
use super::receiver::SolverEventReceiver;
use super::runtime::{panic_payload_to_string, SolverRuntime};
#[cfg(test)]
use super::slot::SLOT_FREE;
use super::slot::{JobSlot, SLOT_SOLVING};
use super::types::{
    SolverEvent, SolverLifecycleState, SolverManagerError, SolverSnapshot, SolverSnapshotAnalysis,
    SolverStatus, SolverTelemetryDetail,
};

/// Maximum concurrent jobs per SolverManager instance.
pub(crate) const MAX_JOBS: usize = 16;

/// Trait for solutions that can run inside the retained lifecycle manager.
pub trait Solvable: PlanningSolution + Send + 'static {
    /// Runs one retained solve, publishing lifecycle events through `runtime`.
    ///
    /// Return `Err` when the solve could not be declared, compiled, prepared
    /// or executed. The manager emits `SolverEvent::Failed` with the error's
    /// message unless the solve already emitted a terminal event. A panic is
    /// treated as a bug and also ends the job with `Failed`.
    fn solve(
        self,
        runtime: SolverRuntime<Self>,
        qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    ) -> RuntimeBuildResult<()>;
}

/// Manages retained async solve jobs with lifecycle-complete event streaming.
pub struct SolverManager<S: Solvable> {
    slots: [JobSlot<S>; MAX_JOBS],
    _phantom: PhantomData<fn() -> S>,
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
    /// Creates a manager with all 16 job slots free. `const`, so a manager can
    /// live in a `static` (the solve methods take `&'static self`).
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
            _phantom: PhantomData,
        }
    }

    /// Starts a retained solve job and returns its id with the job's event
    /// stream.
    pub fn solve(
        &'static self,
        solution: S,
    ) -> Result<(usize, SolverEventReceiver<S>), SolverManagerError> {
        self.submit(solution, None)
    }

    /// Submits a retained solve with externally validated candidate-trace provenance.
    ///
    /// This changes only the diagnostic trace header. Job allocation, worker
    /// execution, event publication, snapshots, and terminalization use the
    /// same retained lifecycle as [`Self::solve`].
    pub fn solve_with_qualified_candidate_trace_provenance(
        &'static self,
        solution: S,
        provenance: QualifiedCandidateTraceRunProvenance,
    ) -> Result<(usize, SolverEventReceiver<S>), SolverManagerError> {
        self.submit(solution, Some(provenance))
    }

    fn submit(
        &'static self,
        solution: S,
        qualified_candidate_trace_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    ) -> Result<(usize, SolverEventReceiver<S>), SolverManagerError> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let Some(slot_idx) = self
            .slots
            .iter()
            .position(|slot| slot.try_initialize(sender.clone()))
        else {
            return Err(SolverManagerError::NoFreeJobSlots);
        };

        let slot = &self.slots[slot_idx];
        let runtime = SolverRuntime::new(slot_idx, slot);

        rayon::spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                solution.solve(runtime, qualified_candidate_trace_provenance)
            }));

            match result {
                Ok(Err(error)) => {
                    if !runtime.is_terminal() {
                        runtime.emit_failed(error.to_string());
                    }
                }
                Ok(Ok(())) => {
                    if !runtime.is_terminal() {
                        if runtime.is_cancel_requested() {
                            let (current_score, best_score, telemetry) =
                                runtime.slot.with_publication(|_, record| {
                                    (
                                        record.current_score,
                                        record.best_score,
                                        record.telemetry.clone(),
                                    )
                                });
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

        Ok((slot_idx, SolverEventReceiver::new(receiver)))
    }

    /// Returns the job's current lifecycle state, scores and compact telemetry.
    ///
    /// # Errors
    ///
    /// [`SolverManagerError::JobNotFound`] when `job_id` names no live job.
    pub fn get_status(&self, job_id: usize) -> Result<SolverStatus<S::Score>, SolverManagerError> {
        let slot = self.slot(job_id)?;
        slot.with_publication(|_, record| {
            let state = slot
                .public_state()
                .ok_or(SolverManagerError::JobNotFound { job_id })?;
            Ok(record.status(job_id, state))
        })
    }

    /// Returns compact aggregate telemetry and bounded candidate-pull detail
    /// from one retained publication instant. The embedded status's scores and
    /// latest snapshot revision identify that same instant.
    ///
    /// This is deliberately separate from [`Self::get_status`], lifecycle
    /// events, and snapshots: those control-plane payloads remain compact as
    /// a solve publishes progress. Detail is returned only when it belongs to
    /// the current compact telemetry publication; it is cleared on every
    /// later trace-free progress, resume, lifecycle-only, or terminal update.
    pub fn get_telemetry_detail(
        &self,
        job_id: usize,
    ) -> Result<SolverTelemetryDetail<S::Score>, SolverManagerError> {
        let slot = self.slot(job_id)?;
        slot.with_publication(|_, record| {
            let state = slot
                .public_state()
                .ok_or(SolverManagerError::JobNotFound { job_id })?;
            Ok(SolverTelemetryDetail {
                status: record.status(job_id, state),
                candidate_trace: record.candidate_trace_detail.clone(),
            })
        })
    }

    /// Requests a pause of a solving job. The job moves to `PauseRequested` and
    /// emits [`SolverEvent::PauseRequested`]; the worker then stops at its next
    /// safe point and emits [`SolverEvent::Paused`], publishing a snapshot when
    /// best-solution publication is enabled.
    ///
    /// # Errors
    ///
    /// [`SolverManagerError::JobNotFound`] for an unknown job, and
    /// [`SolverManagerError::InvalidStateTransition`] unless the job is `Solving`.
    pub fn pause(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        let paused = slot.with_publication(|sender, record| {
            match slot.state.compare_exchange(
                SLOT_SOLVING,
                super::slot::SLOT_PAUSE_REQUESTED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    slot.control
                        .fetch_or(super::slot::CONTROL_PAUSE, Ordering::SeqCst);
                    record.clear_candidate_trace_detail();
                    let metadata =
                        record.next_metadata(job_id, SolverLifecycleState::PauseRequested, None);
                    if let Some(sender) = sender {
                        let _ = sender.send(SolverEvent::PauseRequested { metadata });
                    }
                    true
                }
                Err(_) => false,
            }
        });
        if paused {
            Ok(())
        } else {
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

    /// Resumes a paused job; the worker emits [`SolverEvent::Resumed`] when it
    /// continues.
    ///
    /// # Errors
    ///
    /// [`SolverManagerError::JobNotFound`] for an unknown job, and
    /// [`SolverManagerError::InvalidStateTransition`] unless the job is `Paused`.
    pub fn resume(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        slot.with_publication(|_, _record| {
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
            Ok(())
        })?;
        slot.clear_pause_and_wake();
        Ok(())
    }

    /// Requests cancellation of a job that is solving, pause-requested or paused
    /// (a paused worker is woken). The worker stops at its next safe point and
    /// the job ends with [`SolverEvent::Cancelled`].
    ///
    /// # Errors
    ///
    /// [`SolverManagerError::JobNotFound`] for an unknown job, and
    /// [`SolverManagerError::InvalidStateTransition`] when the job is already
    /// terminal.
    pub fn cancel(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        slot.with_publication(|_, _record| {
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
            Ok(())
        })?;
        slot.clear_pause_and_wake();
        Ok(())
    }

    /// Removes a terminal job, closing its event stream and freeing its slot once
    /// the worker has exited.
    ///
    /// # Errors
    ///
    /// [`SolverManagerError::JobNotFound`] for an unknown or already deleted job,
    /// and [`SolverManagerError::InvalidStateTransition`] while the job is not
    /// terminal.
    pub fn delete(&self, job_id: usize) -> Result<(), SolverManagerError> {
        let slot = self.slot(job_id)?;
        match slot.delete_terminal() {
            Ok(()) => {}
            Err(None) => return Err(SolverManagerError::JobNotFound { job_id }),
            Err(Some(state)) => {
                return Err(SolverManagerError::InvalidStateTransition {
                    job_id,
                    action: "delete",
                    state,
                });
            }
        }
        slot.try_reset_deleted();
        Ok(())
    }

    /// Returns a retained snapshot of the job: the one with `snapshot_revision`,
    /// or the latest when `None`. Best-solution, pause and completion
    /// publications each retain a snapshot with the next revision, starting at 1.
    ///
    /// # Errors
    ///
    /// [`SolverManagerError::JobNotFound`] for an unknown job,
    /// [`SolverManagerError::NoSnapshotAvailable`] before the first snapshot, and
    /// [`SolverManagerError::SnapshotNotFound`] for a revision that was never
    /// retained.
    pub fn get_snapshot(
        &self,
        job_id: usize,
        snapshot_revision: Option<u64>,
    ) -> Result<SolverSnapshot<S>, SolverManagerError> {
        let slot = self.slot(job_id)?;
        slot.with_publication(|_, record| {
            if slot.public_state().is_none() {
                return Err(SolverManagerError::JobNotFound { job_id });
            }

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
        })
    }

    /// Runs [`Analyzable::analyze`] on a retained snapshot (the latest when
    /// `snapshot_revision` is `None`).
    ///
    /// # Errors
    ///
    /// The same errors as [`Self::get_snapshot`].
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

    /// Number of slots holding a visible job, terminal or not; deleted jobs are
    /// not counted.
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

    pub(super) fn slot(&self, job_id: usize) -> Result<&JobSlot<S>, SolverManagerError> {
        self.slots
            .get(job_id)
            .ok_or(SolverManagerError::JobNotFound { job_id })
    }
}
