use std::marker::PhantomData;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::Ordering;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use tokio::sync::mpsc;

use super::super::solution_manager::Analyzable;
use super::runtime::{panic_payload_to_string, SolverRuntime};
use super::slot::{JobSlot, SLOT_FREE, SLOT_SOLVING};
use super::types::{
    SolverEvent, SolverLifecycleState, SolverManagerError, SolverSnapshot, SolverSnapshotAnalysis,
    SolverStatus,
};

/// Maximum concurrent jobs per SolverManager instance.
pub const MAX_JOBS: usize = 16;

/// Trait for solutions that can run inside the retained lifecycle manager.
pub trait Solvable: PlanningSolution + Send + 'static {
    fn solve(self, runtime: SolverRuntime<Self>);
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
        let paused = slot.with_publication(|sender, record| {
            match slot.state.compare_exchange(
                SLOT_SOLVING,
                super::slot::SLOT_PAUSE_REQUESTED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    slot.pause_requested.store(true, Ordering::SeqCst);
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
