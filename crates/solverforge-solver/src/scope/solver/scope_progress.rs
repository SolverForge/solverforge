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

    pub(crate) fn report_phase_progress(&self, phase: crate::stats::PhaseTelemetry) {
        let mut telemetry = self.stats.snapshot_without_applied_move_trace();
        telemetry.phase = Some(phase);
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::Progress,
            status: self.progress_state(),
            solution: None,
            current_score: self.current_score.as_ref(),
            best_score: self.best_score.as_ref(),
            telemetry,
        });
    }

    pub fn report_best_solution(&self) {
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
        if self.start_time.is_none() {
            self.start_solving();
        }
        self.current_score = Some(score);
        self.best_solution = Some(solution);
        self.best_score = Some(score);
        self.last_best_elapsed = self.elapsed();
        self.best_solution_revision = Some(self.solution_revision);
        self.observe_phase_score(score, self.total_step_count);
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

    pub(crate) fn mandatory_control_polling_required(&self) -> bool {
        self.yielded_to_parent || self.terminate.is_some() || self.runtime.is_some()
    }

    pub(crate) fn mandatory_construction_pending_control(&self) -> PendingControl {
        if self.yielded_to_parent || self.is_terminate_early() {
            return PendingControl::CancelRequested;
        }
        if self
            .runtime
            .is_some_and(|runtime| runtime.is_pause_requested())
        {
            return PendingControl::PauseRequested;
        }
        PendingControl::Continue
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

    pub(crate) fn mandatory_construction_work_should_stop(&self) -> bool {
        self.yielded_to_parent
            || self.is_terminate_early()
            || self
                .runtime
                .is_some_and(|runtime| runtime.is_pause_requested())
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

    pub(crate) fn should_interrupt_mandatory_construction(&mut self) -> bool {
        self.settle_pause_if_requested();
        if self.yielded_to_parent {
            return true;
        }
        if self.is_terminate_early() {
            self.mark_cancelled();
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

    /// Enables the one bounded, core-owned candidate-pull recorder for this
    /// solve. Normal progress snapshots intentionally omit this recorder.
    pub(crate) fn enable_candidate_trace(
        &mut self,
        header: CandidateTraceHeader,
        max_entries: usize,
    ) {
        self.stats
            .enable_candidate_trace(CandidateTraceTelemetry::new(header, max_entries));
    }

    pub(crate) fn begin_candidate_trace_phase(&mut self) -> Option<usize> {
        self.stats.begin_candidate_trace_phase()
    }

    /// Publishes the executor-supplied terminal phase plan into the enabled
    /// candidate trace. This is intentionally a direct forwarding boundary:
    /// the scope never infers a plan from progress, snapshots, or callbacks.
    pub(crate) fn finalize_candidate_trace_resolved_phase_plan(
        &mut self,
        resolved_phase_plan: crate::stats::CandidateTracePhasePlan,
    ) {
        self.stats
            .finalize_candidate_trace_resolved_phase_plan(resolved_phase_plan);
    }

    /// Records one engine-consumed cursor candidate when tracing is enabled.
    ///
    /// The recorder decides capacity before asking the move for its owned
    /// canonical identity. Once the bounded prefix is full, it still counts
    /// pulls but allocates neither identities nor trace entries.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_candidate_pull<M>(
        &mut self,
        source: CandidateTraceSource,
        phase_index: usize,
        phase_type: &'static str,
        step_index: u64,
        selector_index: Option<usize>,
        candidate_index: usize,
        construction_target: Option<CandidateTraceConstructionTarget>,
        candidate: &M,
    ) -> Option<CandidateTracePullToken>
    where
        M: Move<S>,
    {
        let CandidateTraceRecordDecision::Capture { ordinal } =
            self.stats.prepare_candidate_trace_pull()
        else {
            return None;
        };
        let identity = candidate.candidate_trace_identity();
        Some(self.stats.record_prepared_candidate_trace_pull(CandidatePullTelemetry {
                ordinal,
                source,
                phase_index,
                phase_type: phase_type.to_string(),
                step_index,
                selector_index,
                candidate_index,
                construction_target,
                identity,
                dispositions: Vec::new(),
            }))
    }

    /// Records one explicit specialized-engine trial that is not represented
    /// by a `MoveCursor`.  The operation identity is still created only when
    /// the bounded recorder has room for it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_candidate_operation<I, T>(
        &mut self,
        source: CandidateTraceSource,
        phase_index: usize,
        phase_type: &'static str,
        step_index: u64,
        selector_index: Option<usize>,
        candidate_index: usize,
        construction_target: Option<CandidateTraceConstructionTarget>,
        descriptor_index: usize,
        operation: &'static str,
        components: I,
    ) -> Option<CandidateTracePullToken>
    where
        I: IntoIterator<Item = T>,
        T: Into<crate::stats::CandidateTraceCoordinate>,
    {
        let CandidateTraceRecordDecision::Capture { ordinal } =
            self.stats.prepare_candidate_trace_pull()
        else {
            return None;
        };
        Some(self.stats.record_prepared_candidate_trace_pull(CandidatePullTelemetry {
                ordinal,
                source,
                phase_index,
                phase_type: phase_type.to_string(),
                step_index,
                selector_index,
                candidate_index,
                construction_target,
                identity: Some(crate::stats::CandidateTraceIdentity::operation(
                    descriptor_index,
                    operation,
                    components,
                )),
                dispositions: Vec::new(),
            }))
    }

    pub(crate) fn record_candidate_trace_disposition(
        &mut self,
        token: CandidateTracePullToken,
        disposition: CandidateTraceDisposition,
    ) {
        self.stats
            .record_candidate_trace_disposition(token, disposition);
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
            .is_some_and(|limit| self.stats.moves_evaluated >= limit)
    }

    fn inphase_score_calc_count_limit_reached(&self) -> bool {
        self.inphase_score_calc_count_limit
            .is_some_and(|limit| self.stats.score_calculations >= limit)
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
