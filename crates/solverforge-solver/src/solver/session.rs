//! A running solve: the solver's phase and termination plus the solver scope.
//!
//! The configured runtime entrypoint starts a session, runs its phase and
//! finishes it, driving the three steps itself so it can
//! switch the score director's constraint weight mode between them: when an
//! applied problem change needs the other mode, the runner returns, `run`
//! reports the switch, `switch_director` moves the whole session onto a
//! director of the other mode, and `run` resumes the phase where it stopped. Every
//! lifecycle step outside the phase (initial best publication, terminal
//! notifications, result extraction) still happens exactly once.

use super::*;

pub(crate) struct SolveSession<'t, P, T, S: PlanningSolution, D: Director<S>, ProgressCb> {
    phase: P,
    termination: T,
    scope: SolverScope<'t, S, D, ProgressCb>,
    // Whether a director switch stopped the phase mid-pass.
    resume: bool,
}

impl<'t, P, T, S, D, ProgressCb> SolveSession<'t, P, T, S, D, ProgressCb>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    /// Moves the session onto the director `rebuild` makes from the current
    /// one; see `SolverScope::switch_score_director`. The termination keeps
    /// its state and only changes the director type it is checked against.
    pub(crate) fn switch_director<D2, T2>(
        self,
        rebuild: impl FnOnce(D) -> D2,
        retarget: impl FnOnce(T) -> T2,
    ) -> SolveSession<'t, P, T2, S, D2, ProgressCb>
    where
        D2: Director<S>,
    {
        SolveSession {
            phase: self.phase,
            termination: retarget(self.termination),
            scope: self.scope.switch_score_director(rebuild),
            resume: self.resume,
        }
    }
}

impl<'t, S, D, T, ProgressCb, P> Solver<'t, P, T, S, D, ProgressCb>
where
    S: PlanningSolution,
    D: Director<S>,
    T: MaybeTermination<S, D, ProgressCb>,
    ProgressCb: ProgressCallback<S>,
    P: Phase<S, D, ProgressCb>,
{
    /// Starts, runs and finishes a session in one call, for solves that
    /// never switch the director's weight mode.
    #[cfg(test)]
    pub(crate) fn solve(self, score_director: D) -> SolveResult<S> {
        let mut session = self.start_session(score_director);
        let switch_requested = session.run();
        debug_assert!(
            !switch_requested,
            "only the configured runtime entrypoint switches weight modes"
        );
        session.finish()
    }

    /// Builds the solver scope and publishes the initial best
    /// solution; `run` then executes the phase.
    pub(crate) fn start_session(
        self,
        score_director: D,
    ) -> SolveSession<'t, P, T, S, D, ProgressCb> {
        let Solver {
            phase,
            termination,
            terminate,
            runtime,
            config,
            candidate_trace_execution_policy,
            candidate_trace_qualified_run_provenance,
            time_limit,
            progress_callback,
            weight_mode_control,
            ..
        } = self;

        let mut solver_scope =
            SolverScope::new_with_callback(score_director, progress_callback, terminate, runtime);
        solver_scope.set_weight_mode_control(weight_mode_control);
        if let Some(trace_config) = config.as_ref().and_then(|config| config.candidate_trace) {
            let resolved_phase_plan = CandidateTracePhasePlan::known(
                "solverforge.solver",
                [("top_level_phase_count", "1".to_string())],
                vec![phase.candidate_trace_plan()],
            );
            let configured_input = config
                .as_ref()
                .expect("candidate trace configuration requires SolverConfig")
                .canonical_toml();
            let execution_policy = candidate_trace_execution_policy.unwrap_or_else(|| {
                CandidateTraceExecutionPolicy::opaque_with_attributes(
                    "solverforge.execution_policy.direct_generic_termination",
                    [(
                        "solver_with_time_limit_ns",
                        time_limit.map_or_else(
                            || "none".to_string(),
                            |limit| limit.as_nanos().to_string(),
                        ),
                    )],
                )
            });
            let header = match candidate_trace_qualified_run_provenance {
                Some(provenance) => CandidateTraceHeader::new_qualified(
                    configured_input,
                    execution_policy,
                    resolved_phase_plan,
                    provenance,
                ),
                None => CandidateTraceHeader::new(
                    configured_input,
                    execution_policy,
                    resolved_phase_plan,
                    None,
                ),
            };
            solver_scope.enable_candidate_trace(header, trace_config.max_entries.get());
        }
        if let Some(environment_mode) = config.as_ref().map(|cfg| cfg.environment_mode) {
            solver_scope = solver_scope.with_environment_mode(environment_mode);
        }
        if let Some(seed) = config.as_ref().and_then(|cfg| cfg.random_seed) {
            solver_scope = solver_scope.with_seed(seed);
        }
        if let Some(limit) = time_limit {
            solver_scope.set_time_limit(limit);
        }
        if phase.defers_initial_best_solution_publication() {
            solver_scope.defer_best_solution_publication();
        }
        solver_scope.initialize_working_solution_as_best();
        solver_scope.report_best_solution();
        solver_scope.pause_if_requested();

        // Install in-phase termination limits so phases can check them
        // inside their step loops (StepCountTermination, BestScoreTermination)
        termination.install_inphase_limits(&mut solver_scope);

        SolveSession {
            phase,
            termination,
            scope: solver_scope,
            resume: false,
        }
    }
}

impl<'t, S, D, T, ProgressCb, P> SolveSession<'t, P, T, S, D, ProgressCb>
where
    S: PlanningSolution,
    D: Director<S>,
    T: MaybeTermination<S, D, ProgressCb>,
    ProgressCb: ProgressCallback<S>,
    P: Phase<S, D, ProgressCb>,
{
    /// Executes the phase with termination checking, resuming it where a
    /// director switch stopped it. Returns `true` when the phase stopped for
    /// a weight-mode switch, before anything else runs.
    pub(crate) fn run(&mut self) -> bool {
        let solver_scope = &mut self.scope;
        if std::mem::take(&mut self.resume) {
            // Re-entered mid-pass: the phase continues exactly where its own
            // loop would, so no boundary check.
            self.phase.solve(solver_scope);
            if solver_scope.weight_mode_switch_pending() {
                self.resume = true;
                return true;
            }
            solver_scope.pause_if_requested();
            return false;
        }
        solver_scope.pause_if_requested();
        if !check_termination(&mut self.termination, solver_scope) {
            tracing::debug!("Starting phase ({})", self.phase.phase_type_name());
            self.phase.solve(solver_scope);
            if solver_scope.weight_mode_switch_pending() {
                self.resume = true;
                return true;
            }
            solver_scope.pause_if_requested();
            tracing::debug!(
                "Finished phase ({}) with score {:?}",
                self.phase.phase_type_name(),
                solver_scope.best_score()
            );
        }
        false
    }

    /// Delivers the terminal notifications and extracts the result.
    pub(crate) fn finish(self) -> SolveResult<S> {
        let SolveSession {
            mut phase,
            mut termination,
            scope: mut solver_scope,
            ..
        } = self;
        // The phase receives one terminal lifecycle notification, even if
        // cancellation or configured termination skipped its solve method.
        phase.on_solver_terminal(&mut solver_scope);
        solver_scope.pause_if_requested();
        let _ = check_termination(&mut termination, &mut solver_scope);

        // Extract solution and stats before consuming scope
        let (solution, current_score, best_score, stats, terminal_reason) =
            solver_scope.take_solution_and_stats();
        SolveResult {
            solution,
            current_score,
            best_score,
            terminal_reason,
            stats,
        }
    }
}
fn check_termination<S, D, ProgressCb, T>(
    termination: &mut T,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    T: MaybeTermination<S, D, ProgressCb>,
{
    if solver_scope.is_terminate_early() {
        solver_scope.mark_cancelled();
        return true;
    }
    if termination.should_terminate(solver_scope) {
        solver_scope.mark_terminated_by_config();
        true
    } else {
        false
    }
}
