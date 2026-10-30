impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    fn problem_change_pending(&self) -> bool {
        self.runtime
            .is_some_and(|runtime| runtime.is_problem_change_pending())
    }

    /// Out-of-line decode of a non-zero runtime control word for
    /// `settle_pause_if_requested`. A pause settles first (a child scope yields
    /// it to its parent); the word is then re-read so changes queued before or
    /// during the pause stop the running work through the existing
    /// `yielded_to_parent` early exits, without a terminal reason. Configured
    /// limits outrank changes, so the yield is skipped while one has fired and
    /// the caller's limit checks record it instead. Only the top-level runner
    /// drains the queue and clears the flag again.
    #[cold]
    #[inline(never)]
    fn settle_runtime_control(&mut self, runtime: SolverRuntime<S>) {
        if runtime.is_pause_requested() {
            match self.publication {
                Publication::Enabled => runtime.pause_if_requested(self),
                Publication::Disabled => {
                    self.yielded_to_parent = true;
                    return;
                }
            }
        }
        if runtime.is_problem_change_pending()
            && !self.is_terminate_early()
            && !self.configured_limit_reached()
        {
            self.yielded_to_parent = true;
        }
    }

    /// Out-of-line decode of a non-zero runtime control word for
    /// `pending_control`: pause first, then configured limits, then queued
    /// problem changes.
    #[cold]
    #[inline(never)]
    fn pending_runtime_control(&self) -> PendingControl {
        if self
            .runtime
            .is_some_and(|runtime| runtime.is_pause_requested())
        {
            return PendingControl::PauseRequested;
        }
        if self.configured_limit_reached() {
            return PendingControl::ConfigTerminationRequested;
        }
        if self.problem_change_pending() {
            return PendingControl::ProblemChangeRequested;
        }
        PendingControl::Continue
    }

    fn configured_limit_reached(&self) -> bool {
        self.time_limit_reached()
            || self.phase_budget_reached()
            || self.phase_termination_reached()
            || self.inphase_best_score_limit_reached()
            || self.inphase_step_count_limit_reached()
            || self.inphase_move_count_limit_reached()
            || self.inphase_score_calc_count_limit_reached()
    }

    /// True when a queued problem change, not cancellation or a configured
    /// limit, is what stopped the running work.
    pub(crate) fn problem_change_interrupt_pending(&self) -> bool {
        self.terminal_reason.is_none() && !self.is_terminate_early() && self.problem_change_pending()
    }

    /// Drains the queued problem changes when this solve still accepts them,
    /// or closes the queue when it does not. Child scopes never drain.
    pub(crate) fn take_problem_changes(&self) -> Option<Vec<Box<dyn ProblemChange<S>>>> {
        if self.publication == Publication::Disabled {
            return None;
        }
        let accepting = self.terminal_reason.is_none() && !self.is_terminate_early();
        self.runtime?.next_problem_change_batch(accepting)
    }

    /// Closes the problem-change queue at the solver's terminal boundary.
    pub(crate) fn close_problem_changes(&self) {
        if self.publication == Publication::Enabled {
            if let Some(runtime) = self.runtime {
                let _ = runtime.next_problem_change_batch(false);
            }
        }
    }

    /// Applies one drained batch in submission order and fully rescores.
    /// A change that installs invalid constraint weights fails before the
    /// rescore binds them. When the changed weights need the other weight
    /// mode, the rescore is deferred to `switch_score_director` and this
    /// returns `None`.
    ///
    /// The batch starts from the published best solution when there is one,
    /// otherwise from the working solution so in-progress construction is
    /// kept. The old best belongs to a different problem and is dropped;
    /// publication stays deferred until the structural completion gate
    /// accepts the changed solution.
    pub(crate) fn apply_problem_changes(
        &mut self,
        changes: Vec<Box<dyn ProblemChange<S>>>,
    ) -> RuntimeBuildResult<Option<S::Score>> {
        let applied = changes.len() as u64;
        let best = if self.best_solution_publication_enabled {
            self.best_solution.take()
        } else {
            None
        };
        self.committed_mutation(|score_director| {
            if let Some(best) = best {
                *score_director.working_solution_mut() = best;
            }
            for change in changes {
                change.apply(&mut *score_director);
            }
        });
        self.score_director.reset();
        // The runner restarts its passes on the changed solution, so the
        // yield that stopped the previous pass is spent.
        self.yielded_to_parent = false;
        self.solution_revision = 1;
        self.construction_frontier.reset();
        self.best_solution = None;
        self.best_score = None;
        self.best_solution_revision = None;
        self.last_best_elapsed = None;
        self.defer_best_solution_publication();
        self.stats.record_problem_changes_applied(applied);
        validate_director_constraint_weights(&self.score_director)?;
        if self.weight_mode_switch_required()? {
            self.weight_mode_switch_pending = true;
            return Ok(None);
        }
        let score = self.calculate_score();
        self.report_progress();
        Ok(Some(score))
    }
}
