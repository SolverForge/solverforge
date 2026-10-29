use std::sync::atomic::Ordering;

use solverforge_core::domain::PlanningSolution;

use super::SolverRuntime;
use crate::manager::solver_manager::slot::CONTROL_PROBLEM_CHANGE;
use crate::realtime::ProblemChange;

impl<S: PlanningSolution> SolverRuntime<S> {
    /// Decodes the problem-change bit of the control word. Safe points only
    /// reach this out of line, after `has_pending_control` saw a request.
    pub(crate) fn is_problem_change_pending(&self) -> bool {
        self.slot.control.load(Ordering::Acquire) & CONTROL_PROBLEM_CHANGE != 0
    }

    /// Drains every queued change in submission order when `accepting`;
    /// otherwise closes the queue. Either way the problem-change bit is
    /// cleared under the same lock that submission sets it under, atomically
    /// with the drain, so a later submission always re-raises it.
    pub(crate) fn next_problem_change_batch(
        &self,
        accepting: bool,
    ) -> Option<Vec<Box<dyn ProblemChange<S>>>> {
        self.slot.with_publication(|_, record| {
            self.slot
                .control
                .fetch_and(!CONTROL_PROBLEM_CHANGE, Ordering::AcqRel);
            record.problem_changes.take_batch(accepting)
        })
    }

    #[cfg(test)]
    pub(crate) fn submit_problem_change_for_test(
        &self,
        change: Box<dyn ProblemChange<S>>,
    ) -> Result<
        crate::manager::solver_manager::problem_change::ProblemChangeId,
        crate::manager::solver_manager::types::SolverManagerError,
    > {
        self.slot.submit_problem_change(self.job_id, change)
    }

    #[cfg(test)]
    pub(crate) fn request_pause_for_test(&self) {
        self.slot.control.fetch_or(
            crate::manager::solver_manager::slot::CONTROL_PAUSE,
            Ordering::SeqCst,
        );
    }
}
