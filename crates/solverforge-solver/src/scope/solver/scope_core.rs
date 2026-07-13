const PROGRESS_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProgressTick {
    pub elapsed: Duration,
    pub step_delta: u64,
    pub move_delta: u64,
}

#[derive(Debug, Clone, Copy)]
struct ProgressPulse {
    phase_index: usize,
    phase_type: &'static str,
    next_deadline: Instant,
    last_reported_at: Instant,
    last_step_count: u64,
    last_move_count: u64,
}

impl ProgressPulse {
    fn new(
        now: Instant,
        phase_index: usize,
        phase_type: &'static str,
        step_count: u64,
        move_count: u64,
    ) -> Self {
        Self {
            phase_index,
            phase_type,
            next_deadline: now + PROGRESS_INTERVAL,
            last_reported_at: now,
            last_step_count: step_count,
            last_move_count: move_count,
        }
    }

    fn take_due(
        &mut self,
        now: Instant,
        phase_index: usize,
        phase_type: &'static str,
        step_count: u64,
        move_count: u64,
    ) -> Option<ProgressTick> {
        if self.phase_index != phase_index || self.phase_type != phase_type {
            *self = Self::new(now, phase_index, phase_type, step_count, move_count);
            return None;
        }
        if now < self.next_deadline {
            return None;
        }
        let tick = ProgressTick {
            elapsed: now.duration_since(self.last_reported_at),
            step_delta: step_count.saturating_sub(self.last_step_count),
            move_delta: move_count.saturating_sub(self.last_move_count),
        };
        self.last_reported_at = now;
        self.last_step_count = step_count;
        self.last_move_count = move_count;
        while self.next_deadline <= now {
            self.next_deadline += PROGRESS_INTERVAL;
        }
        Some(tick)
    }
}

pub struct SolverScope<'t, S: PlanningSolution, D: Director<S>, ProgressCb = ()> {
    score_director: D,
    best_solution: Option<S>,
    current_score: Option<S::Score>,
    best_score: Option<S::Score>,
    rng: StdRng,
    start_time: Option<Instant>,
    paused_at: Option<Instant>,
    total_step_count: u64,
    terminate: Option<&'t AtomicBool>,
    runtime: Option<SolverRuntime<S>>,
    publication: Publication,
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
    construction_frontier: ConstructionFrontier,
    phase_budget: Option<&'t PhaseBudget>,
    pub inphase_step_count_limit: Option<u64>,
    pub inphase_move_count_limit: Option<u64>,
    pub inphase_score_calc_count_limit: Option<u64>,
    inphase_best_score_limit: Option<S::Score>,
    phase_termination: Option<ScopedPhaseTermination<S>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Publication {
    Enabled,
    Disabled,
}

pub(crate) struct PhaseBudget {
    step_count_limit: Option<u64>,
    move_count_limit: Option<u64>,
    score_calc_count_limit: Option<u64>,
    step_count: AtomicU64,
    moves_evaluated: AtomicU64,
    score_calculations: AtomicU64,
}

impl PhaseBudget {
    fn from_scope<S, D, ProgressCb>(scope: &SolverScope<'_, S, D, ProgressCb>) -> Self
    where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        Self {
            step_count_limit: remaining_limit(
                scope.inphase_step_count_limit,
                scope.total_step_count,
            ),
            move_count_limit: remaining_limit(
                scope.inphase_move_count_limit,
                scope.stats.moves_evaluated,
            ),
            score_calc_count_limit: remaining_limit(
                scope.inphase_score_calc_count_limit,
                scope.stats.score_calculations,
            ),
            step_count: AtomicU64::new(0),
            moves_evaluated: AtomicU64::new(0),
            score_calculations: AtomicU64::new(0),
        }
    }

    fn has_limits(&self) -> bool {
        self.step_count_limit.is_some()
            || self.move_count_limit.is_some()
            || self.score_calc_count_limit.is_some()
    }

    fn record_step(&self) {
        self.step_count.fetch_add(1, Ordering::SeqCst);
    }

    fn record_evaluated_move(&self) {
        self.moves_evaluated.fetch_add(1, Ordering::SeqCst);
    }

    fn record_score_calculation(&self) {
        self.score_calculations.fetch_add(1, Ordering::SeqCst);
    }

    fn limit_reached(&self) -> bool {
        limit_reached(self.step_count_limit, self.step_count.load(Ordering::SeqCst))
            || limit_reached(
                self.move_count_limit,
                self.moves_evaluated.load(Ordering::SeqCst),
            )
            || limit_reached(
                self.score_calc_count_limit,
                self.score_calculations.load(Ordering::SeqCst),
            )
    }
}

#[derive(Clone, Copy)]
struct ScopedPhaseTermination<S: PlanningSolution> {
    start_step_count: u64,
    start_elapsed: Duration,
    time_limit: Option<Duration>,
    step_count_limit: Option<u64>,
    best_score_limit: Option<S::Score>,
    unimproved_step_count_limit: Option<u64>,
    unimproved_time_limit: Option<Duration>,
    best_score: Option<S::Score>,
    improvement_score: Option<S::Score>,
    last_improvement_step_count: u64,
    last_improvement_elapsed: Duration,
}

impl<S> ScopedPhaseTermination<S>
where
    S: PlanningSolution,
{
    fn from_config<D, ProgressCb>(
        scope: &SolverScope<'_, S, D, ProgressCb>,
        config: &TerminationConfig,
    ) -> Option<Self>
    where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
        S::Score: ParseableScore,
    {
        let best_score_limit = config
            .best_score_limit
            .as_deref()
            .and_then(|score| S::Score::parse(score).ok());
        let time_limit = config.time_limit();
        let step_count_limit = config.step_count_limit;
        let unimproved_step_count_limit = config.unimproved_step_count_limit;
        let unimproved_time_limit = config.unimproved_time_limit();
        if time_limit.is_none()
            && step_count_limit.is_none()
            && best_score_limit.is_none()
            && unimproved_step_count_limit.is_none()
            && unimproved_time_limit.is_none()
        {
            return None;
        }
        let elapsed = scope.elapsed().unwrap_or_default();
        let best_score = match (scope.best_score, scope.current_score) {
            (Some(best), Some(current)) => Some(best.max(current)),
            (Some(best), None) | (None, Some(best)) => Some(best),
            (None, None) => None,
        };
        Some(Self {
            start_step_count: scope.total_step_count,
            start_elapsed: elapsed,
            time_limit,
            step_count_limit,
            best_score_limit,
            unimproved_step_count_limit,
            unimproved_time_limit,
            best_score,
            improvement_score: scope.current_score.or(scope.best_score),
            last_improvement_step_count: scope.total_step_count,
            last_improvement_elapsed: elapsed,
        })
    }

    fn is_reached(
        &self,
        total_step_count: u64,
        elapsed: Duration,
    ) -> bool {
        let phase_steps = total_step_count.saturating_sub(self.start_step_count);
        let phase_elapsed = elapsed.saturating_sub(self.start_elapsed);
        self.time_limit.is_some_and(|limit| phase_elapsed >= limit)
            || self.step_count_limit.is_some_and(|limit| phase_steps >= limit)
            || self
                .best_score_limit
                .is_some_and(|limit| self.best_score.is_some_and(|score| score >= limit))
            || self.unimproved_step_count_limit.is_some_and(|limit| {
                total_step_count.saturating_sub(self.last_improvement_step_count) >= limit
            })
            || self.unimproved_time_limit.is_some_and(|limit| {
                elapsed.saturating_sub(self.last_improvement_elapsed) >= limit
            })
    }

    fn record_improvement(&mut self, total_step_count: u64, elapsed: Duration) {
        self.last_improvement_step_count = total_step_count;
        self.last_improvement_elapsed = elapsed;
    }

    fn observe_score(
        &mut self,
        score: S::Score,
        completed_step_count: u64,
        elapsed: Duration,
    ) {
        if self.best_score.is_none_or(|best| score > best) {
            self.best_score = Some(score);
        }
        if self.improvement_score.is_none_or(|best| score > best) {
            self.improvement_score = Some(score);
            self.record_improvement(completed_step_count, elapsed);
        }
    }

    fn needs_score_observation(&self) -> bool {
        self.best_score_limit.is_some()
            || self.unimproved_step_count_limit.is_some()
            || self.unimproved_time_limit.is_some()
    }
}

fn remaining_limit(limit: Option<u64>, used: u64) -> Option<u64> {
    limit.map(|limit| limit.saturating_sub(used))
}

fn limit_reached(limit: Option<u64>, used: u64) -> bool {
    limit.is_some_and(|limit| used >= limit)
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
}

impl<'t, S: PlanningSolution, D: Director<S>> SolverScope<'t, S, D, ()> {
    pub fn new(score_director: D) -> Self {
        let construction_frontier = ConstructionFrontier::new();
        Self {
            score_director,
            best_solution: None,
            current_score: None,
            best_score: None,
            rng: StdRng::from_rng(&mut rand::rng()),
            start_time: None,
            paused_at: None,
            total_step_count: 0,
            terminate: None,
            runtime: None,
            publication: Publication::Enabled,
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
            construction_frontier,
            phase_budget: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
            inphase_best_score_limit: None,
            phase_termination: None,
        }
    }
}

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
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
            start_time: None,
            paused_at: None,
            total_step_count: 0,
            terminate,
            runtime,
            publication: Publication::Enabled,
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
            construction_frontier,
            phase_budget: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
            inphase_best_score_limit: None,
            phase_termination: None,
        }
    }

    pub fn with_terminate(mut self, terminate: Option<&'t AtomicBool>) -> Self {
        self.terminate = terminate;
        self
    }

    pub fn with_runtime(mut self, runtime: Option<SolverRuntime<S>>) -> Self {
        self.runtime = runtime;
        self
    }

    pub(crate) fn without_publication(mut self) -> Self {
        self.publication = Publication::Disabled;
        self
    }

    pub(crate) fn yielded_to_parent(&self) -> bool {
        self.yielded_to_parent
    }

    pub fn with_environment_mode(mut self, environment_mode: EnvironmentMode) -> Self {
        self.environment_mode = environment_mode;
        self
    }

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
            termination.is_reached(self.total_step_count, self.elapsed().unwrap_or_default())
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
                self.start_time
                    .map(|start| start + limit)
                    .unwrap_or_else(|| Instant::now() + limit)
            })
        })
    }

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
            start_time: self.start_time,
            paused_at: self.paused_at,
            total_step_count: self.total_step_count,
            terminate: self.terminate,
            runtime: self.runtime,
            publication: self.publication,
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
            construction_frontier: self.construction_frontier,
            phase_budget: self.phase_budget,
            inphase_step_count_limit: self.inphase_step_count_limit,
            inphase_move_count_limit: self.inphase_move_count_limit,
            inphase_score_calc_count_limit: self.inphase_score_calc_count_limit,
            inphase_best_score_limit: self.inphase_best_score_limit,
            phase_termination: self.phase_termination,
        }
    }

    pub fn start_solving(&mut self) {
        self.start_time = Some(Instant::now());
        self.paused_at = None;
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

    pub fn elapsed(&self) -> Option<Duration> {
        match (self.start_time, self.paused_at) {
            (Some(start), Some(paused_at)) => Some(paused_at.duration_since(start)),
            (Some(start), None) => Some(start.elapsed()),
            _ => None,
        }
    }

    pub fn time_since_last_improvement(&self) -> Option<Duration> {
        let elapsed = self.elapsed()?;
        let last_best_elapsed = self.last_best_elapsed?;
        Some(elapsed.saturating_sub(last_best_elapsed))
    }

    pub fn score_director(&self) -> &D {
        &self.score_director
    }

    pub(crate) fn score_director_mut(&mut self) -> &mut D {
        &mut self.score_director
    }

    pub fn working_solution(&self) -> &S {
        self.score_director.working_solution()
    }

    pub fn mutate<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.committed_mutation(mutate)
    }

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

    pub fn initialize_working_solution_as_best(&mut self) -> S::Score {
        if self.start_time.is_none() {
            self.start_solving();
        }
        let score = self.calculate_score();
        let solution = self.score_director.clone_working_solution();
        self.set_best_solution(solution, score);
        score
    }

    pub fn replace_working_solution_and_reinitialize(&mut self, solution: S) -> S::Score {
        *self.score_director.working_solution_mut() = solution;
        self.score_director.reset();
        self.current_score = None;
        self.best_solution_revision = None;
        self.solution_revision = 1;
        self.construction_frontier.reset();
        self.calculate_score()
    }

    pub fn best_solution(&self) -> Option<&S> {
        self.best_solution.as_ref()
    }

    pub fn best_score(&self) -> Option<&S::Score> {
        self.best_score.as_ref()
    }

    pub fn current_score(&self) -> Option<&S::Score> {
        self.current_score.as_ref()
    }

    pub(crate) fn is_scalar_slot_completed(&self, slot_id: ConstructionSlotId) -> bool {
        self.construction_frontier
            .is_scalar_completed(slot_id, self.solution_revision)
    }

    pub(crate) fn mark_scalar_slot_completed(&mut self, slot_id: ConstructionSlotId) {
        self.construction_frontier
            .mark_scalar_completed(slot_id, self.solution_revision);
    }

    pub(crate) fn is_group_slot_completed(&self, slot_id: &ConstructionGroupSlotId) -> bool {
        self.construction_frontier
            .is_group_completed(slot_id, self.solution_revision)
    }

    pub(crate) fn mark_group_slot_completed(&mut self, slot_id: ConstructionGroupSlotId) {
        self.construction_frontier
            .mark_group_completed(slot_id, self.solution_revision);
    }

    pub(crate) fn is_list_element_completed(&self, element_id: ConstructionListElementId) -> bool {
        self.construction_frontier
            .is_list_completed(element_id, self.solution_revision)
    }

    pub(crate) fn mark_list_element_completed(&mut self, element_id: ConstructionListElementId) {
        self.construction_frontier
            .mark_list_completed(element_id, self.solution_revision);
    }

    #[cfg(test)]
    pub(crate) fn solution_revision(&self) -> u64 {
        self.solution_revision
    }

    pub(crate) fn apply_committed_move<M>(&mut self, mov: &M)
    where
        M: Move<S>,
    {
        self.committed_mutation(|score_director| mov.do_move(score_director));
    }

    pub(crate) fn apply_committed_change<F>(&mut self, change: F)
    where
        F: FnOnce(&mut D),
    {
        self.committed_mutation(change);
    }

}
