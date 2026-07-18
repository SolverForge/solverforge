use std::sync::atomic::Ordering;

use solverforge_core::domain::PlanningSolution;

use super::{EventKind, SolverRuntime};
use crate::manager::solver_manager::slot::SLOT_PAUSED;
use crate::manager::{SolverEvent, SolverLifecycleState};
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::SolverTelemetry;

impl<S: PlanningSolution> SolverRuntime<S> {
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
        let current_score = solver_scope.current_score().copied();
        let telemetry = solver_scope.stats().snapshot();
        if solver_scope.best_solution_publication_enabled() {
            let solution = solver_scope.score_director().clone_working_solution();
            let best_score = solver_scope.best_score().copied();
            let _ = self.pause_with_snapshot(solution, current_score, best_score, telemetry);
        } else {
            let _ = self.pause_without_snapshot(current_score, telemetry);
        }
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

        let (telemetry, candidate_trace) = telemetry.split_candidate_trace();
        let published = self.slot.with_publication(|sender, record| {
            if !self.slot.pause_requested.load(Ordering::Acquire)
                || self.is_cancel_requested()
                || self.current_state().is_terminal()
            {
                return false;
            }
            self.slot.state.store(SLOT_PAUSED, Ordering::SeqCst);
            let terminal_reason = record.terminal_reason;
            record.checkpoint_available = true;
            record.current_score = current_score;
            record.best_score = best_score;
            record.publish_telemetry_with_candidate_trace(telemetry.clone(), candidate_trace);

            let snapshot_revision = record.push_snapshot(crate::manager::SolverSnapshot {
                job_id: self.job_id,
                snapshot_revision: 0,
                lifecycle_state: SolverLifecycleState::Paused,
                terminal_reason,
                current_score,
                best_score,
                telemetry: record.telemetry.clone(),
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
            true
        });
        if !published {
            return false;
        }
        self.wait_for_resume(current_score, best_score, telemetry)
    }

    fn pause_without_snapshot(
        &self,
        current_score: Option<S::Score>,
        telemetry: SolverTelemetry,
    ) -> bool {
        let (telemetry, candidate_trace) = telemetry.split_candidate_trace();
        let published = self.slot.with_publication(|sender, record| {
            if !self.slot.pause_requested.load(Ordering::Acquire)
                || self.is_cancel_requested()
                || self.current_state().is_terminal()
            {
                return false;
            }
            self.slot.state.store(SLOT_PAUSED, Ordering::SeqCst);
            record.checkpoint_available = false;
            record.current_score = current_score;
            record.best_score = None;
            record.publish_telemetry_with_candidate_trace(telemetry.clone(), candidate_trace);
            let metadata = record.next_metadata(self.job_id, SolverLifecycleState::Paused, None);
            if let Some(sender) = sender {
                let _ = sender.send(SolverEvent::Paused { metadata });
            }
            true
        });
        if !published {
            return false;
        }
        self.wait_for_resume(current_score, None, telemetry)
    }

    fn wait_for_resume(
        &self,
        current_score: Option<S::Score>,
        best_score: Option<S::Score>,
        telemetry: SolverTelemetry,
    ) -> bool {
        let mut guard = self.slot.pause_gate.lock().unwrap();
        while self.slot.pause_requested.load(Ordering::Acquire) && !self.is_cancel_requested() {
            guard = self.slot.pause_condvar.wait(guard).unwrap();
        }
        drop(guard);
        if self.is_cancel_requested() {
            return false;
        }
        self.emit_non_snapshot_event(current_score, best_score, telemetry, EventKind::Resumed);
        true
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::{Duration, Instant};

    use solverforge_core::score::SoftScore;

    use super::*;
    use crate::manager::solver_manager::slot::{JobSlot, SLOT_SOLVING};

    #[derive(Clone, Debug)]
    struct PauseTestSolution {
        score: Option<SoftScore>,
    }

    impl PlanningSolution for PauseTestSolution {
        type Score = SoftScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[test]
    fn incomplete_pause_exposes_no_checkpoint_or_solution_snapshot() {
        let slot = Box::leak(Box::new(JobSlot::<PauseTestSolution>::new()));
        slot.state.store(SLOT_SOLVING, Ordering::Release);
        slot.pause_requested.store(true, Ordering::Release);
        let runtime = SolverRuntime::new(7, slot);
        let worker = thread::spawn(move || {
            runtime.pause_without_snapshot(Some(SoftScore::of(0)), SolverTelemetry::default())
        });

        let deadline = Instant::now() + Duration::from_secs(2);
        while slot.state.load(Ordering::Acquire) != SLOT_PAUSED {
            assert!(Instant::now() < deadline, "pause did not settle");
            thread::yield_now();
        }
        {
            let record = slot.record.lock().unwrap();
            assert!(!record.checkpoint_available);
            assert!(record.latest_snapshot_revision.is_none());
            assert!(record.snapshots.is_empty());
            assert!(record.best_score.is_none());
        }

        slot.pause_requested.store(false, Ordering::Release);
        slot.pause_condvar.notify_all();
        assert!(worker.join().expect("pause worker must resume"));
    }
}
