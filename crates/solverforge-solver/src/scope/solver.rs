// Solver-level scope.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::{Director, RecordingDirector};

use crate::heuristic::r#move::Move;
use crate::manager::{SolverLifecycleState, SolverRuntime, SolverTerminalReason};
use crate::phase::construction::{
    ConstructionFrontier, ConstructionListElementId, ConstructionSlotId,
};
use crate::stats::SolverStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverProgressKind {
    Progress,
    BestSolution,
}

#[derive(Debug, Clone, Copy)]
pub struct SolverProgressRef<'a, S: PlanningSolution> {
    pub kind: SolverProgressKind,
    pub status: SolverLifecycleState,
    pub solution: Option<&'a S>,
    pub current_score: Option<&'a S::Score>,
    pub best_score: Option<&'a S::Score>,
    pub telemetry: crate::stats::SolverTelemetry,
}

pub trait ProgressCallback<S: PlanningSolution>: Send + Sync {
    fn invoke(&self, progress: SolverProgressRef<'_, S>);
}

impl<S: PlanningSolution> ProgressCallback<S> for () {
    fn invoke(&self, _progress: SolverProgressRef<'_, S>) {}
}

impl<S, F> ProgressCallback<S> for F
where
    S: PlanningSolution,
    F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync,
{
    fn invoke(&self, progress: SolverProgressRef<'_, S>) {
        self(progress);
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
        PendingControl::Continue
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
        if self.is_terminate_early() {
            self.mark_cancelled();
            return true;
        }
        if self.time_limit_reached() {
            self.mark_terminated_by_config();
            return true;
        }
        false
    }

    pub fn should_terminate(&mut self) -> bool {
        self.settle_pause_if_requested();
        if self.is_terminate_early() {
            self.mark_cancelled();
            return true;
        }
        if self.time_limit_reached() {
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
            runtime.pause_if_requested(self);
        }
    }

    fn time_limit_reached(&self) -> bool {
        self.time_limit
            .zip(self.elapsed())
            .is_some_and(|(limit, elapsed)| elapsed >= limit)
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
