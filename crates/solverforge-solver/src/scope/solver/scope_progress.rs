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
        if !ProgressCb::PUBLISHES_PROGRESS {
            return;
        }
        let best_score = self
            .best_solution_publication_enabled
            .then_some(self.best_score.as_ref())
            .flatten();
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::Progress,
            status: self.progress_state(),
            solution: None,
            current_score: self.current_score.as_ref(),
            best_score,
            telemetry: self.stats.snapshot_without_applied_move_trace(),
        });
    }

    pub(crate) fn begin_phase_progress(
        &mut self,
        phase_index: usize,
        phase_type: &'static str,
        step_count: u64,
        move_count: u64,
    ) {
        self.progress_pulse = Some(ProgressPulse::new(
            Instant::now(),
            phase_index,
            phase_type,
            step_count,
            move_count,
        ));
    }

    pub(crate) fn take_phase_progress_tick(
        &mut self,
        phase_index: usize,
        phase_type: &'static str,
        step_count: u64,
        move_count: u64,
    ) -> Option<ProgressTick> {
        let now = Instant::now();
        let pulse = self.progress_pulse.get_or_insert_with(|| {
            ProgressPulse::new(now, phase_index, phase_type, step_count, move_count)
        });
        pulse.take_due(now, phase_index, phase_type, step_count, move_count)
    }

    pub(crate) fn report_phase_progress(&self) {
        if !ProgressCb::PUBLISHES_PROGRESS {
            return;
        }
        let best_score = self
            .best_solution_publication_enabled
            .then_some(self.best_score.as_ref())
            .flatten();
        let telemetry = self.stats.snapshot_without_applied_move_trace();
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::Progress,
            status: self.progress_state(),
            solution: None,
            current_score: self.current_score.as_ref(),
            best_score,
            telemetry,
        });
    }

    pub fn report_best_solution(&self) {
        if !ProgressCb::PUBLISHES_PROGRESS || !self.best_solution_publication_enabled {
            return;
        }
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::BestSolution,
            status: self.progress_state(),
            solution: self.best_solution.as_ref(),
            current_score: self.current_score.as_ref(),
            best_score: self.best_score.as_ref(),
            telemetry: self.stats.snapshot_without_applied_move_trace(),
        });
    }

    pub fn update_best_solution(&mut self) {
        let current_score = self.score_director.calculate_score();
        self.current_score = Some(current_score);
        self.assert_score_consistent("update_best_solution", current_score);
        self.observe_phase_score(current_score, self.total_step_count);
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
        if self.stats.started_at().is_none() {
            self.start_solving();
        }
        self.current_score = Some(score);
        self.best_solution = Some(solution);
        self.best_score = Some(score);
        self.last_best_elapsed = self.elapsed();
        self.best_solution_revision = Some(self.solution_revision);
        self.observe_phase_score(score, self.total_step_count);
    }

    pub(crate) fn publish_current_solution_as_best(&mut self) {
        self.retain_current_solution_as_best();
        self.best_solution_publication_enabled = true;
        self.report_best_solution();
    }

    pub(crate) fn retain_current_solution_as_best(&mut self) {
        let score = self.calculate_score();
        let solution = self.score_director.clone_working_solution();
        self.set_best_solution(solution, score);
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

    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }

    pub fn pause_if_requested(&mut self) {
        self.settle_pause_if_requested();
    }

    pub fn pause_timers(&mut self) {
        self.stats.pause();
    }

    pub fn resume_timers(&mut self) {
        self.stats.resume();
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

    pub fn mark_cancelled(&mut self) {
        self.terminal_reason
            .get_or_insert(SolverTerminalReason::Cancelled);
    }

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

    pub fn stats(&self) -> &SolverStats {
        &self.stats
    }

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
