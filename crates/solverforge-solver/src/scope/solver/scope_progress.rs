impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    /// Why the solve stopped; `Completed` unless cancellation or a configured
    /// limit was recorded.
    pub fn terminal_reason(&self) -> SolverTerminalReason {
        self.terminal_reason
            .unwrap_or(SolverTerminalReason::Completed)
    }

    /// Sets the current score without recalculating it.
    pub fn set_current_score(&mut self, score: S::Score) {
        self.current_score = Some(score);
    }

    /// Publishes a progress report through the progress callback; a no-op for
    /// the `()` callback.
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

    /// Publishes the best solution through the progress callback; a no-op for
    /// the `()` callback or while best-solution publication is deferred.
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

    /// Scores the working solution and, when it beats the best score (or no
    /// best exists), clones it as the new best solution and publishes it.
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

    /// Records `solution` with `score` as the best solution (and the current
    /// score), starting the solve clock if needed. Does not publish it.
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

    pub(crate) fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// A uniformly random index below `upper`, drawn from the solve's seeded
    /// generator so a fixed `with_seed` reproduces it.
    ///
    /// # Panics
    /// When `upper` is zero.
    pub fn random_below(&mut self, upper: usize) -> usize {
        rand::RngExt::random_range(&mut self.rng, 0..upper)
    }

    /// A uniformly random `u64` from the solve's seeded generator.
    pub fn random_u64(&mut self) -> u64 {
        rand::RngExt::random(&mut self.rng)
    }

    /// Increments the solve's total step count and returns the new count.
    pub fn increment_step_count(&mut self) -> u64 {
        self.total_step_count += 1;
        self.stats.record_step();
        if let Some(phase_budget) = self.phase_budget {
            phase_budget.record_step();
        }
        self.total_step_count
    }

    /// Steps completed across all phases of the solve.
    pub fn total_step_count(&self) -> u64 {
        self.total_step_count
    }

    /// Consumes the scope and returns the best solution, if any.
    pub fn take_best_solution(self) -> Option<S> {
        self.best_solution
    }

    /// Consumes the scope and returns the best solution, or a clone of the
    /// working solution when none was recorded.
    pub fn take_best_or_working_solution(self) -> S {
        self.best_solution
            .unwrap_or_else(|| self.score_director.clone_working_solution())
    }

    /// Consumes the scope and returns the best (or working) solution, the
    /// current score, the best score, the statistics and the terminal reason.
    ///
    /// # Panics
    ///
    /// Panics when no score was ever calculated.
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
}
