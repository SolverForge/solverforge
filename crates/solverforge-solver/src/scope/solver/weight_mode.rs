/* Constraint weight mode of the running score director across problem changes.

A score director's weight mode is a type, so a solve cannot switch it in
place. When an applied problem change needs the other mode, the scope records
the switch instead of rescoring; the runner returns, and the solve entrypoint
rebuilds the director in the other mode and moves this scope's whole state
onto it with `switch_score_director`, which then performs the deferred rescore.
*/

/// How a solve manages its director's constraint weight mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WeightModeControl {
    /// The director keeps its mode; problem changes never switch it.
    Fixed,
    /// The unweighted entrypoint: a change that requires weighting fails the
    /// solve.
    UnweightedOnly,
    /// The weighted entrypoint, currently running a director of the given
    /// mode: a change that needs the other mode switches the director.
    Switchable { weighted: bool },
}

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    pub(crate) fn set_weight_mode_control(&mut self, control: WeightModeControl) {
        self.weight_mode = control;
    }

    /// True after an applied problem change that the running director's
    /// weight mode cannot score; the solve entrypoint must switch directors.
    pub(crate) fn weight_mode_switch_pending(&self) -> bool {
        self.weight_mode_switch_pending
    }

    /* Whether the working solution, just rewritten by a problem change, needs
    the other weight mode. Constant `false` for solution types without
    constraint weights. */
    fn weight_mode_switch_required(&self) -> RuntimeBuildResult<bool> {
        if !S::HAS_CONSTRAINT_WEIGHTS {
            return Ok(false);
        }
        let required = requires_weighted_mode(self.working_solution());
        match self.weight_mode {
            WeightModeControl::Fixed => Ok(false),
            WeightModeControl::UnweightedOnly if required => {
                Err(RuntimeBuildError::ConstraintWeights {
                    message: "a problem change installed constraint weights on a solve that \
                              scores unweighted; solve this solution type through \
                              `try_run_weighted_solver_with_config_and_search`"
                        .to_string(),
                })
            }
            WeightModeControl::UnweightedOnly => Ok(false),
            WeightModeControl::Switchable { weighted } => Ok(required != weighted),
        }
    }

    /// Moves this scope onto the director `rebuild` makes from the current
    /// one, keeping every other piece of solve state: statistics and clock,
    /// configured limits, RNG, candidate trace, runtime and publication state.
    /// Completes the rescore the problem change that requested the switch
    /// deferred.
    pub(crate) fn switch_score_director<D2: Director<S>>(
        self,
        rebuild: impl FnOnce(D) -> D2,
    ) -> SolverScope<'t, S, D2, ProgressCb> {
        debug_assert!(self.weight_mode_switch_pending);
        let weight_mode = match self.weight_mode {
            WeightModeControl::Switchable { weighted } => WeightModeControl::Switchable {
                weighted: !weighted,
            },
            control => control,
        };
        let mut scope = SolverScope {
            score_director: rebuild(self.score_director),
            best_solution: self.best_solution,
            current_score: self.current_score,
            best_score: self.best_score,
            rng: self.rng,
            total_step_count: self.total_step_count,
            terminate: self.terminate,
            runtime: self.runtime,
            publication: self.publication,
            best_solution_publication_enabled: self.best_solution_publication_enabled,
            yielded_to_parent: self.yielded_to_parent,
            environment_mode: self.environment_mode,
            stats: self.stats,
            time_limit: self.time_limit,
            time_deadline: self.time_deadline,
            progress_callback: self.progress_callback,
            progress_pulse: self.progress_pulse,
            terminal_reason: self.terminal_reason,
            last_best_elapsed: self.last_best_elapsed,
            best_solution_revision: self.best_solution_revision,
            solution_revision: self.solution_revision,
            working_solution_generation: self.working_solution_generation,
            construction_frontier: self.construction_frontier,
            phase_budget: self.phase_budget,
            inphase_step_count_limit: self.inphase_step_count_limit,
            inphase_move_count_limit: self.inphase_move_count_limit,
            inphase_score_calc_count_limit: self.inphase_score_calc_count_limit,
            inphase_best_score_limit: self.inphase_best_score_limit,
            phase_termination: self.phase_termination,
            weight_mode,
            weight_mode_switch_pending: false,
        };
        tracing::debug!(
            event = "constraint_weight_mode_switch",
            weighted = matches!(weight_mode, WeightModeControl::Switchable { weighted: true }),
        );
        scope.calculate_score();
        scope.report_progress();
        scope
    }
}
