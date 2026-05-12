impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    pub fn terminal_reason(&self) -> SolverTerminalReason {
        self.terminal_reason
            .unwrap_or(SolverTerminalReason::Completed)
    }

    pub fn set_current_score(&mut self, score: S::Score) {
        self.current_score = Some(score);
    }

    pub fn report_progress(&self) {
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::Progress,
            status: self.progress_state(),
            solution: None,
            current_score: self.current_score.as_ref(),
            best_score: self.best_score.as_ref(),
            telemetry: self.stats.snapshot(),
        });
    }

    pub fn report_best_solution(&self) {
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::BestSolution,
            status: self.progress_state(),
            solution: self.best_solution.as_ref(),
            current_score: self.current_score.as_ref(),
            best_score: self.best_score.as_ref(),
            telemetry: self.stats.snapshot(),
        });
    }

    pub fn update_best_solution(&mut self) {
        let current_score = self.score_director.calculate_score();
        self.current_score = Some(current_score);
        self.assert_score_consistent("update_best_solution", current_score);
        let is_better = match &self.best_score {
            None => true,
            Some(best) => current_score > *best,
        };

        if is_better {
            self.best_solution = Some(self.score_director.clone_working_solution());
            self.best_score = Some(current_score);
            self.last_best_elapsed = self.elapsed();
            self.best_solution_revision = Some(self.solution_revision);
            self.report_best_solution();
        }
    }

    pub(crate) fn promote_current_solution_on_score_tie(&mut self) {
        let Some(current_score) = self.current_score else {
            return;
        };
        let Some(best_score) = self.best_score else {
            return;
        };

        if current_score == best_score
            && self.best_solution_revision != Some(self.solution_revision)
        {
            self.best_solution = Some(self.score_director.clone_working_solution());
            self.best_solution_revision = Some(self.solution_revision);
            self.report_best_solution();
        }
    }

    pub fn set_best_solution(&mut self, solution: S, score: S::Score) {
        if self.start_time.is_none() {
            self.start_solving();
        }
        self.current_score = Some(score);
        self.best_solution = Some(solution);
        self.best_score = Some(score);
        self.last_best_elapsed = self.elapsed();
        self.best_solution_revision = Some(self.solution_revision);
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    pub fn increment_step_count(&mut self) -> u64 {
        self.total_step_count += 1;
        self.stats.record_step();
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_step();
        }
        self.total_step_count
    }

    pub fn total_step_count(&self) -> u64 {
        self.total_step_count
    }

    pub fn take_best_solution(self) -> Option<S> {
        self.best_solution
    }

    pub fn take_best_or_working_solution(self) -> S {
        self.best_solution
            .unwrap_or_else(|| self.score_director.clone_working_solution())
    }

    pub fn take_solution_and_stats(
        self,
    ) -> (
        S,
        Option<S::Score>,
        S::Score,
        SolverStats,
        SolverTerminalReason,
    ) {
        let terminal_reason = self.terminal_reason();
        let solution = self
            .best_solution
            .unwrap_or_else(|| self.score_director.clone_working_solution());
        let best_score = self
            .best_score
            .or(self.current_score)
            .expect("solver finished without a canonical score");
        (
            solution,
            self.current_score,
            best_score,
            self.stats,
            terminal_reason,
        )
    }

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
            .is_some_and(|runtime| runtime.is_pause_requested())
        {
            return PendingControl::PauseRequested;
        }
        if self.time_limit_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        if self.phase_budget_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        PendingControl::Continue
    }

    pub(crate) fn work_should_stop(&self) -> bool {
        self.yielded_to_parent
            || self.is_terminate_early()
            || self.time_limit_reached()
            || self.phase_budget_reached()
    }

    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }

    pub fn pause_if_requested(&mut self) {
        self.settle_pause_if_requested();
    }

    pub fn pause_timers(&mut self) {
        if self.paused_at.is_none() {
            self.paused_at = Some(Instant::now());
            self.stats.pause();
        }
    }

    pub fn resume_timers(&mut self) {
        if let Some(paused_at) = self.paused_at.take() {
            let paused_for = paused_at.elapsed();
            if let Some(start) = self.start_time {
                self.start_time = Some(start + paused_for);
            }
            self.stats.resume();
        }
    }

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
        false
    }

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
        if let Some(limit) = self.inphase_step_count_limit {
            if self.total_step_count >= limit {
                self.mark_terminated_by_config();
                return true;
            }
        }
        if let Some(limit) = self.inphase_move_count_limit {
            if self.stats.moves_evaluated >= limit {
                self.mark_terminated_by_config();
                return true;
            }
        }
        if let Some(limit) = self.inphase_score_calc_count_limit {
            if self.stats.score_calculations >= limit {
                self.mark_terminated_by_config();
                return true;
            }
        }
        false
    }

    pub fn mark_cancelled(&mut self) {
        self.terminal_reason
            .get_or_insert(SolverTerminalReason::Cancelled);
    }

    pub fn mark_terminated_by_config(&mut self) {
        self.terminal_reason
            .get_or_insert(SolverTerminalReason::TerminatedByConfig);
    }

    pub fn stats(&self) -> &SolverStats {
        &self.stats
    }

    pub fn stats_mut(&mut self) -> &mut SolverStats {
        &mut self.stats
    }

    pub(crate) fn record_evaluated_move(&mut self, duration: Duration) {
        self.stats.record_evaluated_move(duration);
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_evaluated_move();
        }
    }

    pub(crate) fn record_selector_evaluated(&mut self, selector_index: usize, duration: Duration) {
        self.stats.record_selector_evaluated(selector_index, duration);
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_evaluated_move();
        }
    }

    pub(crate) fn record_score_calculation(&mut self) {
        self.stats.record_score_calculation();
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_score_calculation();
        }
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
            if !runtime.is_pause_requested() || self.is_terminate_early() {
                return;
            }
            match self.publication {
                Publication::Enabled => runtime.pause_if_requested(self),
                Publication::Disabled => {
                    self.yielded_to_parent = true;
                }
            }
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
        output
    }
}
