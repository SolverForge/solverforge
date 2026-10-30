/* Cancellation, pause and configured-limit checks of the solver scope, its
statistics accessors and the committed-mutation bookkeeping. Included by
`scope/solver.rs`. */

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    /// Whether cancellation was requested through the flag or the runtime.
    pub fn is_terminate_early(&self) -> bool {
        self.terminate
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
            || self
                .runtime
                .is_some_and(|runtime| runtime.is_cancel_requested())
    }

    pub(crate) fn pending_control(&self) -> PendingControl {
        if self.is_terminate_early() {
            return PendingControl::CancelRequested;
        }
        if self
            .runtime
            .is_some_and(|runtime| runtime.has_pending_control())
        {
            return self.pending_runtime_control();
        }
        if self.time_limit_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        if self.phase_budget_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        if self.phase_termination_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        if self.inphase_best_score_limit_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        if self.inphase_step_count_limit_reached()
            || self.inphase_move_count_limit_reached()
            || self.inphase_score_calc_count_limit_reached()
        {
            return PendingControl::ConfigTerminationRequested;
        }
        PendingControl::Continue
    }

    pub(crate) fn config_control_polling_required(&self) -> bool {
        self.yielded_to_parent
            || self.terminate.is_some()
            || self.runtime.is_some()
            || self.time_limit.is_some()
            || self.time_deadline.is_some()
            || self.phase_budget.is_some()
            || self.phase_termination.is_some()
            || self.inphase_best_score_limit.is_some()
            || self.inphase_step_count_limit.is_some()
            || self.inphase_move_count_limit.is_some()
            || self.inphase_score_calc_count_limit.is_some()
    }

    pub(crate) fn work_should_stop(&self) -> bool {
        self.yielded_to_parent
            || self.is_terminate_early()
            || self.time_limit_reached()
            || self.phase_budget_reached()
            || self.phase_termination_reached()
            || self.inphase_best_score_limit_reached()
            || self.inphase_step_count_limit_reached()
            || self.inphase_move_count_limit_reached()
            || self.inphase_score_calc_count_limit_reached()
    }

    /// Stops the solve once this much solver-active time has elapsed.
    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }

    /// Settles a requested pause at this point, blocking until resume or
    /// cancel.
    pub fn pause_if_requested(&mut self) {
        self.settle_pause_if_requested();
    }

    /// Pauses the solve clock.
    pub fn pause_timers(&mut self) {
        self.stats.pause();
    }

    /// Resumes the solve clock.
    pub fn resume_timers(&mut self) {
        self.stats.resume();
    }

    /// Settles a pending pause, then returns whether construction must stop:
    /// the scope yielded to its parent, cancellation (recorded as `Cancelled`),
    /// a time, budget, best-score, step, move or score-calculation limit
    /// (recorded as `TerminatedByConfig`), or a phase-relative termination.
    pub fn should_terminate_construction(&mut self) -> bool {
        self.settle_pause_if_requested();
        if self.yielded_to_parent {
            return true;
        }
        if self.is_terminate_early() {
            self.mark_cancelled();
            return true;
        }
        if self.time_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.phase_budget_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.phase_termination_reached() {
            return true;
        }
        if self.inphase_best_score_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.inphase_step_count_limit_reached()
            || self.inphase_move_count_limit_reached()
            || self.inphase_score_calc_count_limit_reached()
        {
            self.mark_terminated_by_config();
            return true;
        }
        false
    }

    /// Settles a pending pause, then returns whether the solve must stop: the
    /// scope yielded to its parent, cancellation (recorded as `Cancelled`), a
    /// time, budget, best-score, step, move or score-calculation limit
    /// (recorded as `TerminatedByConfig`), or a phase-relative termination.
    pub fn should_terminate(&mut self) -> bool {
        self.settle_pause_if_requested();
        if self.yielded_to_parent {
            return true;
        }
        if self.is_terminate_early() {
            self.mark_cancelled();
            return true;
        }
        if self.time_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.phase_budget_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.phase_termination_reached() {
            return true;
        }
        if self.inphase_best_score_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.inphase_step_count_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.inphase_move_count_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        if self.inphase_score_calc_count_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        false
    }

    /// Records `Cancelled` as the terminal reason unless one is already set.
    pub fn mark_cancelled(&mut self) {
        self.terminal_reason
            .get_or_insert(SolverTerminalReason::Cancelled);
    }

    /// Records `TerminatedByConfig` as the terminal reason unless one is
    /// already set.
    pub fn mark_terminated_by_config(&mut self) {
        self.terminal_reason
            .get_or_insert(SolverTerminalReason::TerminatedByConfig);
    }

    pub(crate) fn install_inphase_best_score_limit(&mut self, target_score: S::Score) {
        let target_score = match self.inphase_best_score_limit {
            Some(existing) => existing.min(target_score),
            None => target_score,
        };
        self.inphase_best_score_limit = Some(target_score);
    }

    /// Solve statistics.
    pub fn stats(&self) -> &SolverStats {
        &self.stats
    }

    /// Solve statistics, mutably.
    pub fn stats_mut(&mut self) -> &mut SolverStats {
        &mut self.stats
    }

    fn progress_state(&self) -> SolverLifecycleState {
        self.runtime
            .map(|runtime| {
                if runtime.is_terminal() {
                    SolverLifecycleState::Completed
                } else {
                    SolverLifecycleState::Solving
                }
            })
            .unwrap_or(SolverLifecycleState::Solving)
    }

    fn settle_pause_if_requested(&mut self) {
        if let Some(runtime) = self.runtime {
            if !runtime.has_pending_control() || self.is_terminate_early() {
                return;
            }
            self.settle_runtime_control(runtime);
        }
    }

    fn time_limit_reached(&self) -> bool {
        if self
            .time_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return true;
        }
        self.time_limit
            .zip(self.elapsed())
            .is_some_and(|(limit, elapsed)| elapsed >= limit)
    }

    fn phase_budget_reached(&self) -> bool {
        self.phase_budget
            .is_some_and(|phase_budget| phase_budget.limit_reached())
    }

    fn inphase_best_score_limit_reached(&self) -> bool {
        self.inphase_best_score_limit
            .zip(self.best_score)
            .is_some_and(|(target, best)| best >= target)
    }

    fn inphase_step_count_limit_reached(&self) -> bool {
        self.inphase_step_count_limit
            .is_some_and(|limit| self.total_step_count >= limit)
    }

    fn inphase_move_count_limit_reached(&self) -> bool {
        self.inphase_move_count_limit
            .is_some_and(|limit| self.stats.moves_evaluated() >= limit)
    }

    fn inphase_score_calc_count_limit_reached(&self) -> bool {
        self.inphase_score_calc_count_limit
            .is_some_and(|limit| self.stats.score_calculations() >= limit)
    }

    fn advance_solution_revision(&mut self) {
        self.solution_revision = self.solution_revision.wrapping_add(1);
        if self.solution_revision == 0 {
            self.solution_revision = 1;
            self.construction_frontier.reset();
        }
    }

    fn committed_mutation<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.current_score = None;
        let output = mutate(&mut self.score_director);
        self.advance_solution_revision();
        self.working_solution_generation += 1;
        output
    }
}
