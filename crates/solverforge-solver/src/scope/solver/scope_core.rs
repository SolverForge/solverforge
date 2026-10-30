/// Solver-wide state of one solve: the score director with the working
/// solution, the best solution and scores, the seeded random generator,
/// termination limits, lifecycle control and statistics.
///
/// `ProgressCb` receives progress and best-solution reports; `()` reports
/// nothing. `'t` bounds the borrowed cancellation flag.
pub struct SolverScope<'t, S: PlanningSolution, D: Director<S>, ProgressCb = ()> {
    score_director: D,
    best_solution: Option<S>,
    current_score: Option<S::Score>,
    best_score: Option<S::Score>,
    rng: StdRng,
    total_step_count: u64,
    terminate: Option<&'t AtomicBool>,
    runtime: Option<SolverRuntime<S>>,
    publication: Publication,
    best_solution_publication_enabled: bool,
    yielded_to_parent: bool,
    environment_mode: EnvironmentMode,
    stats: SolverStats,
    time_limit: Option<Duration>,
    time_deadline: Option<Instant>,
    progress_callback: ProgressCb,
    progress_pulse: Option<ProgressPulse>,
    terminal_reason: Option<SolverTerminalReason>,
    last_best_elapsed: Option<Duration>,
    best_solution_revision: Option<u64>,
    solution_revision: u64,
    /* Monotonic count of working-solution writes: every committed mutation
    and every wholesale replacement. Unlike `solution_revision`, it never
    resets, so evaluator replicas can detect any write since they synced.
    */
    working_solution_generation: u64,
    construction_frontier: ConstructionFrontier,
    phase_budget: Option<&'t PhaseBudget>,
    /// Stops the solve once the total step count reaches this many steps;
    /// `None` means no limit.
    pub inphase_step_count_limit: Option<u64>,
    /// Stops the solve once this many moves were evaluated; `None` means no
    /// limit.
    pub inphase_move_count_limit: Option<u64>,
    /// Stops the solve once this many score calculations ran; `None` means no
    /// limit.
    pub inphase_score_calc_count_limit: Option<u64>,
    inphase_best_score_limit: Option<S::Score>,
    phase_termination: Option<ScopedPhaseTermination<S>>,
    weight_mode: WeightModeControl,
    weight_mode_switch_pending: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct SolverScopeChildConfig<'t, S: PlanningSolution> {
    terminate: Option<&'t AtomicBool>,
    runtime: Option<SolverRuntime<S>>,
    environment_mode: EnvironmentMode,
    time_deadline: Option<Instant>,
    phase_budget: Option<&'t PhaseBudget>,
    inphase_step_count_limit: Option<u64>,
    inphase_move_count_limit: Option<u64>,
    inphase_score_calc_count_limit: Option<u64>,
    inphase_best_score_limit: Option<S::Score>,
}

impl<'t, S: PlanningSolution> SolverScopeChildConfig<'t, S> {
    pub(crate) fn build_scope<PD>(&self, score_director: PD, seed: u64) -> SolverScope<'t, S, PD>
    where
        PD: Director<S>,
    {
        let terminate = self
            .terminate
            .or_else(|| self.runtime.map(|runtime| runtime.cancel_flag()));
        let mut scope = SolverScope::new(score_director)
            .with_terminate(terminate)
            .with_runtime(self.runtime)
            .without_publication()
            .with_environment_mode(self.environment_mode)
            .with_seed(seed);
        scope.time_deadline = self.time_deadline;
        scope.phase_budget = self.phase_budget;
        if self.phase_budget.is_none() {
            scope.inphase_step_count_limit = self.inphase_step_count_limit;
            scope.inphase_move_count_limit = self.inphase_move_count_limit;
            scope.inphase_score_calc_count_limit = self.inphase_score_calc_count_limit;
        }
        scope.inphase_best_score_limit = self.inphase_best_score_limit;
        scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingControl {
    Continue,
    PauseRequested,
    CancelRequested,
    ConfigTerminationRequested,
    ProblemChangeRequested,
}

impl<'t, S: PlanningSolution, D: Director<S>> SolverScope<'t, S, D, ()> {
    /// Creates a scope over `score_director` with no progress callback, an
    /// entropy-seeded random generator and no limits.
    pub fn new(score_director: D) -> Self {
        let construction_frontier = ConstructionFrontier::new();
        Self {
            score_director,
            best_solution: None,
            current_score: None,
            best_score: None,
            rng: StdRng::from_rng(&mut rand::rng()),
            total_step_count: 0,
            terminate: None,
            runtime: None,
            publication: Publication::Enabled,
            best_solution_publication_enabled: true,
            yielded_to_parent: false,
            environment_mode: EnvironmentMode::default(),
            stats: SolverStats::default(),
            time_limit: None,
            time_deadline: None,
            progress_callback: (),
            progress_pulse: None,
            terminal_reason: None,
            last_best_elapsed: None,
            best_solution_revision: None,
            solution_revision: 1,
            working_solution_generation: 0,
            construction_frontier,
            phase_budget: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
            inphase_best_score_limit: None,
            phase_termination: None,
            weight_mode: WeightModeControl::Fixed,
            weight_mode_switch_pending: false,
        }
    }
}

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    /// Creates a scope over `score_director` with a progress `callback`, an
    /// optional cancellation flag and an optional retained runtime for
    /// lifecycle control.
    pub fn new_with_callback(
        score_director: D,
        callback: ProgressCb,
        terminate: Option<&'t AtomicBool>,
        runtime: Option<SolverRuntime<S>>,
    ) -> Self {
        let construction_frontier = ConstructionFrontier::new();
        Self {
            score_director,
            best_solution: None,
            current_score: None,
            best_score: None,
            rng: StdRng::from_rng(&mut rand::rng()),
            total_step_count: 0,
            terminate,
            runtime,
            publication: Publication::Enabled,
            best_solution_publication_enabled: true,
            yielded_to_parent: false,
            environment_mode: EnvironmentMode::default(),
            stats: SolverStats::default(),
            time_limit: None,
            time_deadline: None,
            progress_callback: callback,
            progress_pulse: None,
            terminal_reason: None,
            last_best_elapsed: None,
            best_solution_revision: None,
            solution_revision: 1,
            working_solution_generation: 0,
            construction_frontier,
            phase_budget: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
            inphase_best_score_limit: None,
            phase_termination: None,
            weight_mode: WeightModeControl::Fixed,
            weight_mode_switch_pending: false,
        }
    }

    /// Sets the cancellation flag; the solve stops once it reads `true`.
    pub fn with_terminate(mut self, terminate: Option<&'t AtomicBool>) -> Self {
        self.terminate = terminate;
        self
    }

    /// Attaches the retained runtime that delivers pause, cancel and problem
    /// changes.
    pub fn with_runtime(mut self, runtime: Option<SolverRuntime<S>>) -> Self {
        self.runtime = runtime;
        self
    }

    pub(crate) fn without_publication(mut self) -> Self {
        self.publication = Publication::Disabled;
        self
    }

    pub(crate) fn defer_best_solution_publication(&mut self) {
        self.best_solution_publication_enabled = false;
    }

    pub(crate) fn best_solution_publication_enabled(&self) -> bool {
        self.best_solution_publication_enabled
    }

    pub(crate) fn yielded_to_parent(&self) -> bool {
        self.yielded_to_parent
    }

    /// Sets the environment mode; `FullAssert` checks every calculated score
    /// against a fresh recalculation.
    pub fn with_environment_mode(mut self, environment_mode: EnvironmentMode) -> Self {
        self.environment_mode = environment_mode;
        self
    }

    /// Seeds the random generator so the solve is reproducible.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = StdRng::seed_from_u64(seed);
        self
    }

    pub(crate) fn child_phase_budget(&self) -> PhaseBudget {
        PhaseBudget::from_scope(self)
    }

    pub(crate) fn child_config<'a>(
        &'a self,
        phase_budget: Option<&'a PhaseBudget>,
    ) -> SolverScopeChildConfig<'a, S> {
        let phase_budget = self
            .phase_budget
            .or_else(|| phase_budget.filter(|budget| budget.has_limits()));
        SolverScopeChildConfig {
            terminate: self.terminate,
            runtime: self.runtime,
            environment_mode: self.environment_mode,
            time_deadline: self.child_time_deadline(),
            phase_budget,
            inphase_step_count_limit: self.inphase_step_count_limit,
            inphase_move_count_limit: self.inphase_move_count_limit,
            inphase_score_calc_count_limit: self.inphase_score_calc_count_limit,
            inphase_best_score_limit: self.inphase_best_score_limit,
        }
    }

    /// Runs one configured phase with a phase-relative termination overlay.
    ///
    /// The overlay is intentionally independent from solver-wide termination:
    /// it starts at this phase boundary, applies every supported
    /// `TerminationConfig` limit, and is restored before the next phase runs.
    /// Mandatory default construction deliberately does not install this
    /// overlay, so its required-completion pass remains interruptible only by
    /// lifecycle control.
    pub(crate) fn with_phase_termination<T>(
        &mut self,
        config: Option<&TerminationConfig>,
        work: impl FnOnce(&mut Self) -> T,
    ) -> T
    where
        S::Score: ParseableScore,
    {
        let previous = self.phase_termination.take();
        self.phase_termination =
            config.and_then(|config| ScopedPhaseTermination::from_config(self, config));
        let result = work(self);
        self.phase_termination = previous;
        result
    }

    fn phase_termination_reached(&self) -> bool {
        self.phase_termination.as_ref().is_some_and(|termination| {
            // Only scoped terminations with clock-based limits need the
            // wall clock; step/budget/score limits run on counters and the
            // control poll calls this per candidate.
            let elapsed = if termination.needs_clock() {
                self.elapsed().unwrap_or_default()
            } else {
                Duration::ZERO
            };
            termination.is_reached(self.total_step_count, elapsed)
        })
    }

    pub(crate) fn phase_termination_requires_score_observation(&self) -> bool {
        self.phase_termination
            .as_ref()
            .is_some_and(ScopedPhaseTermination::needs_score_observation)
    }

    fn observe_phase_score(&mut self, score: S::Score, completed_step_count: u64) {
        let elapsed = self.elapsed().unwrap_or_default();
        if let Some(termination) = &mut self.phase_termination {
            termination.observe_score(score, completed_step_count, elapsed);
        }
    }

    pub(crate) fn observe_phase_step_score(&mut self, score: S::Score) {
        self.observe_phase_score(score, self.total_step_count.saturating_add(1));
    }

    fn child_time_deadline(&self) -> Option<Instant> {
        self.time_deadline.or_else(|| {
            self.time_limit.map(|limit| {
                self.stats
                    .started_at()
                    .map(|start| start + limit)
                    .unwrap_or_else(|| Instant::now() + limit)
            })
        })
    }

    /// Replaces the progress callback, keeping all other state.
    pub fn with_progress_callback<F: ProgressCallback<S>>(
        self,
        callback: F,
    ) -> SolverScope<'t, S, D, F> {
        SolverScope {
            score_director: self.score_director,
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
            progress_callback: callback,
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
            weight_mode: self.weight_mode,
            weight_mode_switch_pending: self.weight_mode_switch_pending,
        }
    }

    /// Resets the per-solve counters and best-solution tracking and starts the
    /// solve clock.
    pub fn start_solving(&mut self) {
        self.total_step_count = 0;
        self.terminal_reason = None;
        self.last_best_elapsed = None;
        self.yielded_to_parent = false;
        self.best_solution_revision = None;
        self.solution_revision = 1;
        self.progress_pulse = None;
        self.construction_frontier.reset();
        self.stats.start();
    }

    /// Solver-active time since the solve started, excluding pauses; `None`
    /// before it started.
    pub fn elapsed(&self) -> Option<Duration> {
        self.stats.active_elapsed()
    }

    /// Solver-active time since the best solution last improved; `None` before
    /// the solve or the first best solution.
    pub fn time_since_last_improvement(&self) -> Option<Duration> {
        let elapsed = self.elapsed()?;
        let last_best_elapsed = self.last_best_elapsed?;
        Some(elapsed.saturating_sub(last_best_elapsed))
    }

    /// The score director holding the working solution.
    pub fn score_director(&self) -> &D {
        &self.score_director
    }

    pub(crate) fn score_director_mut(&mut self) -> &mut D {
        &mut self.score_director
    }

    /// The working solution.
    pub fn working_solution(&self) -> &S {
        self.score_director.working_solution()
    }

    /// Applies a committed change to the working solution through the score
    /// director and returns the closure's result. The cached current score is
    /// cleared and the working-solution revision advances.
    pub fn mutate<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.committed_mutation(mutate)
    }

    /// Calculates the working solution's score, records the calculation and
    /// stores it as the current score.
    ///
    /// # Panics
    ///
    /// In `FullAssert` mode, panics when the score differs from a fresh
    /// recalculation.
    pub fn calculate_score(&mut self) -> S::Score {
        self.record_score_calculation();
        let score = self.score_director.calculate_score();
        self.current_score = Some(score);
        self.assert_score_consistent("calculate_score", score);
        score
    }

    pub(crate) fn assert_score_consistent(&self, context: &str, score: S::Score) {
        if self.environment_mode != EnvironmentMode::FullAssert {
            return;
        }
        let Some(fresh_score) = self.score_director.fresh_score() else {
            return;
        };
        assert_eq!(
            score, fresh_score,
            "score director drift after {context}: cached score {score:?} != fresh score {fresh_score:?}"
        );
    }

    /// Starts the solve if needed, scores the working solution and records it
    /// as the best solution. Returns the score.
    pub fn initialize_working_solution_as_best(&mut self) -> S::Score {
        if self.stats.started_at().is_none() {
            self.start_solving();
        }
        let score = self.calculate_score();
        let solution = self.score_director.clone_working_solution();
        self.set_best_solution(solution, score);
        score
    }

    /// Replaces the working solution, resets the score director and the
    /// construction progress, and returns the new solution's score.
    pub fn replace_working_solution_and_reinitialize(&mut self, solution: S) -> S::Score {
        self.working_solution_generation += 1;
        *self.score_director.working_solution_mut() = solution;
        self.score_director.reset();
        self.current_score = None;
        self.best_solution_revision = None;
        self.solution_revision = 1;
        self.construction_frontier.reset();
        self.calculate_score()
    }

    /// The best solution so far; `None` before one was recorded.
    pub fn best_solution(&self) -> Option<&S> {
        self.best_solution.as_ref()
    }

    /// Score of the best solution; `None` before one was recorded.
    pub fn best_score(&self) -> Option<&S::Score> {
        self.best_score.as_ref()
    }

    /// Latest calculated score of the working solution; `None` after a
    /// mutation until the next calculation.
    pub fn current_score(&self) -> Option<&S::Score> {
        self.current_score.as_ref()
    }

}
