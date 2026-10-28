//! Real-time problem-change submission for retained jobs.

use std::collections::VecDeque;
use std::sync::atomic::Ordering;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use super::manager::{Solvable, SolverManager};
use super::slot::{JobSlot, CONTROL_PROBLEM_CHANGE};
use super::types::{SolverLifecycleState, SolverManagerError};
use crate::realtime::ProblemChange;

/// Acknowledges one problem change queued on a retained job.
///
/// The id is the 1-based submission ordinal within its job. The change has
/// been applied once a later event or status reports
/// `telemetry.problem_changes_applied >= id.get()`. A job that reaches a
/// terminal state with a lower count discarded the change because
/// cancellation or a configured limit stopped the solve first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProblemChangeId(u64);

impl ProblemChangeId {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// FIFO of submitted changes owned by one job record.
///
/// Every access happens under the slot's publication lock. The queue closes
/// when the configured runtime finishes its last phase pass, so a late
/// submission is rejected instead of being stranded.
pub(super) struct ProblemChangeQueue<S: PlanningSolution> {
    pending: VecDeque<Box<dyn ProblemChange<S>>>,
    submitted: u64,
    closed: bool,
}

impl<S: PlanningSolution> ProblemChangeQueue<S> {
    pub(super) const fn new() -> Self {
        Self {
            pending: VecDeque::new(),
            submitted: 0,
            closed: false,
        }
    }

    pub(super) fn reset(&mut self) {
        self.pending.clear();
        self.submitted = 0;
        self.closed = false;
    }

    /// Drains the whole FIFO when the solve still accepts changes; otherwise
    /// closes the queue and discards anything left in it.
    pub(super) fn take_batch(&mut self, accepting: bool) -> Option<Vec<Box<dyn ProblemChange<S>>>> {
        if accepting && !self.pending.is_empty() {
            return Some(self.pending.drain(..).collect());
        }
        self.closed = true;
        if !self.pending.is_empty() {
            tracing::warn!(
                discarded = self.pending.len(),
                "discarding problem changes queued after the solve stopped accepting them"
            );
            self.pending.clear();
        }
        None
    }

    fn push(&mut self, change: Box<dyn ProblemChange<S>>) -> ProblemChangeId {
        self.submitted += 1;
        self.pending.push_back(change);
        ProblemChangeId(self.submitted)
    }
}

impl<S: PlanningSolution> JobSlot<S> {
    /// Queues one change under the publication lock, the same lock that
    /// serializes pause, cancel, and every lifecycle publication.
    pub(super) fn submit_problem_change(
        &self,
        job_id: usize,
        change: Box<dyn ProblemChange<S>>,
    ) -> Result<ProblemChangeId, SolverManagerError> {
        self.with_publication(|_, record| {
            let state = self
                .public_state()
                .ok_or(SolverManagerError::JobNotFound { job_id })?;
            let accepting = matches!(
                state,
                SolverLifecycleState::Solving
                    | SolverLifecycleState::PauseRequested
                    | SolverLifecycleState::Paused
            ) && !self.terminate.load(Ordering::SeqCst)
                && !record.problem_changes.closed;
            if !accepting {
                return Err(SolverManagerError::InvalidStateTransition {
                    job_id,
                    action: "add_problem_change",
                    state,
                });
            }
            let id = record.problem_changes.push(change);
            self.control
                .fetch_or(CONTROL_PROBLEM_CHANGE, Ordering::AcqRel);
            Ok(id)
        })
    }
}

impl<S: Solvable> SolverManager<S>
where
    S::Score: Score,
{
    /// Queues a problem change on a running, pause-requested, or paused job.
    ///
    /// The configured runtime applies queued changes in submission order at
    /// its next safe point: it starts from the current best solution, applies
    /// every change, rescores from scratch, and restarts its phase sequence so
    /// construction assigns new or unassigned planning work. A paused job
    /// applies its queue after `resume`.
    ///
    /// Returns `InvalidStateTransition` once cancellation was requested, the
    /// job is terminal, or its final phase pass has finished.
    pub fn add_problem_change<P>(
        &self,
        job_id: usize,
        change: P,
    ) -> Result<ProblemChangeId, SolverManagerError>
    where
        P: ProblemChange<S> + 'static,
    {
        self.slot(job_id)?
            .submit_problem_change(job_id, Box::new(change))
    }
}
