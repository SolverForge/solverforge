// Solver-level scope.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::manager::SolverStatus;
use crate::stats::SolverStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverProgressKind {
    Progress,
    BestSolution,
}

#[derive(Debug, Clone, Copy)]
pub struct SolverProgressRef<'a, S: PlanningSolution> {
    pub kind: SolverProgressKind,
    pub status: SolverStatus,
    pub solution: Option<&'a S>,
    pub score: Option<&'a S::Score>,
    pub telemetry: crate::stats::SolverTelemetry,
}

/// Sealed trait for invoking an optional progress callback.
///
/// Implemented for `()` (no-op) and for any `F: for<'a> Fn(SolverProgressRef<'a, S>) + Send + Sync`.
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

/// Top-level scope for the entire solving process.
///
/// Holds the working solution, score director, and tracks the best solution found.
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag reference
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `ProgressCb` - The progress callback type (default `()` means no callback)
pub struct SolverScope<'t, S: PlanningSolution, D: Director<S>, ProgressCb = ()> {
    // The score director managing the working solution.
    score_director: D,
    // The best solution found so far.
    best_solution: Option<S>,
    // The score of the best solution.
    best_score: Option<S::Score>,
    // Random number generator for stochastic algorithms.
    rng: StdRng,
    // When solving started.
    start_time: Option<Instant>,
    // Total number of steps across all phases.
    total_step_count: u64,
    // Flag for early termination requests.
    terminate: Option<&'t AtomicBool>,
    // Solver statistics.
    stats: SolverStats,
    // Time limit for solving (checked by phases).
    time_limit: Option<Duration>,
    // Callback invoked when the solver should publish progress.
    progress_callback: ProgressCb,
    // Optional maximum total step count for in-phase termination (T1).
    pub inphase_step_count_limit: Option<u64>,
    // Optional maximum total move count for in-phase termination (T1).
    pub inphase_move_count_limit: Option<u64>,
    // Optional maximum total score calculation count for in-phase termination (T1).
    pub inphase_score_calc_count_limit: Option<u64>,
}

impl<'t, S: PlanningSolution, D: Director<S>> SolverScope<'t, S, D, ()> {
    pub fn new(score_director: D) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::from_rng(&mut rand::rng()),
            start_time: None,
            total_step_count: 0,
            terminate: None,
            stats: SolverStats::default(),
            time_limit: None,
            progress_callback: (),
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
        terminate: Option<&'t std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::from_rng(&mut rand::rng()),
            start_time: None,
            total_step_count: 0,
            terminate,
            stats: SolverStats::default(),
            time_limit: None,
            progress_callback: callback,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
        }
    }
}

impl<'t, S: PlanningSolution, D: Director<S>, ProgressCb: ProgressCallback<S>>
    SolverScope<'t, S, D, ProgressCb>
{
    pub fn with_terminate(mut self, terminate: Option<&'t AtomicBool>) -> Self {
        self.terminate = terminate;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = StdRng::seed_from_u64(seed);
        self
    }

    /// Sets the progress callback, transitioning to a typed callback scope.
    ///
    /// The callback is invoked for exact progress updates and best-solution updates.
    pub fn with_progress_callback<F: ProgressCallback<S>>(
        self,
        callback: F,
    ) -> SolverScope<'t, S, D, F> {
        SolverScope {
            score_director: self.score_director,
            best_solution: self.best_solution,
            best_score: self.best_score,
            rng: self.rng,
            start_time: self.start_time,
            total_step_count: self.total_step_count,
            terminate: self.terminate,
            stats: self.stats,
            time_limit: self.time_limit,
            progress_callback: callback,
            inphase_step_count_limit: self.inphase_step_count_limit,
            inphase_move_count_limit: self.inphase_move_count_limit,
            inphase_score_calc_count_limit: self.inphase_score_calc_count_limit,
        }
    }

    /// Marks the start of solving.
    pub fn start_solving(&mut self) {
        self.start_time = Some(Instant::now());
        self.total_step_count = 0;
        self.stats.start();
    }

    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    pub fn score_director(&self) -> &D {
        &self.score_director
    }

    pub fn score_director_mut(&mut self) -> &mut D {
        &mut self.score_director
    }

    pub fn working_solution(&self) -> &S {
        self.score_director.working_solution()
    }

    pub fn working_solution_mut(&mut self) -> &mut S {
        self.score_director.working_solution_mut()
    }

    /// Calculates and returns the current score.
    ///
    /// Also records the score calculation in solver statistics.
    pub fn calculate_score(&mut self) -> S::Score {
        self.stats.record_score_calculation();
        self.score_director.calculate_score()
    }

    pub fn best_solution(&self) -> Option<&S> {
        self.best_solution.as_ref()
    }

    pub fn best_score(&self) -> Option<&S::Score> {
        self.best_score.as_ref()
    }

    pub fn report_progress(&self) {
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::Progress,
            status: SolverStatus::Solving,
            solution: self.best_solution.as_ref(),
            score: self.best_score.as_ref(),
            telemetry: self.stats.snapshot(),
        });
    }

    pub fn report_best_solution(&self) {
        self.progress_callback.invoke(SolverProgressRef {
            kind: SolverProgressKind::BestSolution,
            status: SolverStatus::Solving,
            solution: self.best_solution.as_ref(),
            score: self.best_score.as_ref(),
            telemetry: self.stats.snapshot(),
        });
    }

    /// Updates the best solution if the current solution is better.
    pub fn update_best_solution(&mut self) {
        let current_score = self.score_director.calculate_score();
        let is_better = match &self.best_score {
            None => true,
            Some(best) => current_score > *best,
        };

        if is_better {
            self.best_solution = Some(self.score_director.clone_working_solution());
            self.best_score = Some(current_score);

            self.report_best_solution();
        }
    }

    /// Forces an update of the best solution regardless of score comparison.
    pub fn set_best_solution(&mut self, solution: S, score: S::Score) {
        self.best_solution = Some(solution);
        self.best_score = Some(score);
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// Increments and returns the total step count.
    pub fn increment_step_count(&mut self) -> u64 {
        self.total_step_count += 1;
        self.stats.record_step();
        self.total_step_count
    }

    pub fn total_step_count(&self) -> u64 {
        self.total_step_count
    }

    /// Extracts the best solution, consuming this scope.
    pub fn take_best_solution(self) -> Option<S> {
        self.best_solution
    }

    pub fn take_best_or_working_solution(self) -> S {
        self.best_solution
            .unwrap_or_else(|| self.score_director.clone_working_solution())
    }

    /// Extracts both the solution and stats, consuming this scope.
    ///
    /// Returns the best solution (or working solution if none) along with
    /// the accumulated solver statistics.
    pub fn take_solution_and_stats(self) -> (S, SolverStats) {
        let solution = self
            .best_solution
            .unwrap_or_else(|| self.score_director.clone_working_solution());
        (solution, self.stats)
    }

    pub fn is_terminate_early(&self) -> bool {
        self.terminate
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }

    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }

    pub fn should_terminate_construction(&self) -> bool {
        // Check external termination flag
        if self.is_terminate_early() {
            return true;
        }
        // Check time limit only
        if let (Some(start), Some(limit)) = (self.start_time, self.time_limit) {
            if start.elapsed() >= limit {
                return true;
            }
        }
        false
    }

    pub fn should_terminate(&self) -> bool {
        // Check external termination flag
        if self.is_terminate_early() {
            return true;
        }
        // Check time limit
        if let (Some(start), Some(limit)) = (self.start_time, self.time_limit) {
            if start.elapsed() >= limit {
                return true;
            }
        }
        // Check in-phase step count limit (T1: StepCountTermination fires inside phase loop)
        if let Some(limit) = self.inphase_step_count_limit {
            if self.total_step_count >= limit {
                return true;
            }
        }
        // Check in-phase move count limit (T1: MoveCountTermination fires inside phase loop)
        if let Some(limit) = self.inphase_move_count_limit {
            if self.stats.moves_evaluated >= limit {
                return true;
            }
        }
        // Check in-phase score calculation count limit
        if let Some(limit) = self.inphase_score_calc_count_limit {
            if self.stats.score_calculations >= limit {
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> &SolverStats {
        &self.stats
    }

    pub fn stats_mut(&mut self) -> &mut SolverStats {
        &mut self.stats
    }
}
