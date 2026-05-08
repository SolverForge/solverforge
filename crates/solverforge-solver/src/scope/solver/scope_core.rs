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
    stats: SolverStats,
    time_limit: Option<Duration>,
    progress_callback: ProgressCb,
    terminal_reason: Option<SolverTerminalReason>,
    last_best_elapsed: Option<Duration>,
    best_solution_revision: Option<u64>,
    solution_revision: u64,
    construction_frontier: ConstructionFrontier,
    pub inphase_step_count_limit: Option<u64>,
    pub inphase_move_count_limit: Option<u64>,
    pub inphase_score_calc_count_limit: Option<u64>,
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
            stats: SolverStats::default(),
            time_limit: None,
            progress_callback: (),
            terminal_reason: None,
            last_best_elapsed: None,
            best_solution_revision: None,
            solution_revision: 1,
            construction_frontier,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
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
            stats: SolverStats::default(),
            time_limit: None,
            progress_callback: callback,
            terminal_reason: None,
            last_best_elapsed: None,
            best_solution_revision: None,
            solution_revision: 1,
            construction_frontier,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
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

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = StdRng::seed_from_u64(seed);
        self
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
            stats: self.stats,
            time_limit: self.time_limit,
            progress_callback: callback,
            terminal_reason: self.terminal_reason,
            last_best_elapsed: self.last_best_elapsed,
            best_solution_revision: self.best_solution_revision,
            solution_revision: self.solution_revision,
            construction_frontier: self.construction_frontier,
            inphase_step_count_limit: self.inphase_step_count_limit,
            inphase_move_count_limit: self.inphase_move_count_limit,
            inphase_score_calc_count_limit: self.inphase_score_calc_count_limit,
        }
    }

    pub fn start_solving(&mut self) {
        self.start_time = Some(Instant::now());
        self.paused_at = None;
        self.total_step_count = 0;
        self.terminal_reason = None;
        self.last_best_elapsed = None;
        self.best_solution_revision = None;
        self.solution_revision = 1;
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

    pub fn trial<T, F>(&mut self, trial: F) -> T
    where
        F: FnOnce(&mut RecordingDirector<'_, S, D>) -> T,
    {
        let mut recording = RecordingDirector::new(&mut self.score_director);
        let output = trial(&mut recording);
        recording.undo_changes();
        output
    }

    pub fn mutate<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.committed_mutation(mutate)
    }

    pub fn calculate_score(&mut self) -> S::Score {
        self.stats.record_score_calculation();
        let score = self.score_director.calculate_score();
        self.current_score = Some(score);
        score
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

    pub(crate) fn construction_frontier(&self) -> &ConstructionFrontier {
        &self.construction_frontier
    }
}
