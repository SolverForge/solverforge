//! Solver-level scope.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::stats::SolverStats;

/// Sealed trait for invoking an optional best-solution callback.
///
/// Implemented for `()` (no-op) and for any `F: Fn(&S) + Send + Sync`.
pub trait BestSolutionCallback<S>: Send + Sync {
    /// Invokes the callback with the given solution, if one is registered.
    fn invoke(&self, solution: &S);
}

impl<S> BestSolutionCallback<S> for () {
    fn invoke(&self, _solution: &S) {}
}

impl<S, F: Fn(&S) + Send + Sync> BestSolutionCallback<S> for F {
    fn invoke(&self, solution: &S) {
        self(solution);
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
/// * `BestCb` - The best-solution callback type (default `()` means no callback)
pub struct SolverScope<'t, S: PlanningSolution, D: Director<S>, BestCb = ()> {
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
    // Callback invoked when the best solution improves.
    best_solution_callback: BestCb,
    /// Optional maximum total step count for in-phase termination (T1).
    pub inphase_step_count_limit: Option<u64>,
    /// Optional maximum total move count for in-phase termination (T1).
    pub inphase_move_count_limit: Option<u64>,
    /// Optional maximum total score calculation count for in-phase termination (T1).
    pub inphase_score_calc_count_limit: Option<u64>,
}

impl<'t, S: PlanningSolution, D: Director<S>> SolverScope<'t, S, D, ()> {
    /// Creates a new solver scope with the given score director and no callback.
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
            best_solution_callback: (),
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
        }
    }
}

impl<'t, S: PlanningSolution, D: Director<S>, BestCb: BestSolutionCallback<S>>
    SolverScope<'t, S, D, BestCb>
{
    /// Creates a new solver scope with the given score director and callback.
    pub fn new_with_callback(
        score_director: D,
        callback: BestCb,
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
            best_solution_callback: callback,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
        }
    }
}

impl<'t, S: PlanningSolution, D: Director<S>, BestCb: BestSolutionCallback<S>>
    SolverScope<'t, S, D, BestCb>
{
    /// Sets the termination flag.
    pub fn with_terminate(mut self, terminate: Option<&'t AtomicBool>) -> Self {
        self.terminate = terminate;
        self
    }

    /// Sets a specific random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng = StdRng::seed_from_u64(seed);
        self
    }

    /// Sets the best solution callback, transitioning to a typed callback scope.
    ///
    /// The callback is invoked whenever the best solution improves during solving.
    pub fn with_best_solution_callback<F: Fn(&S) + Send + Sync>(
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
            best_solution_callback: callback,
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

    /// Returns the elapsed time since solving started.
    pub fn elapsed(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    /// Returns a reference to the score director.
    pub fn score_director(&self) -> &D {
        &self.score_director
    }

    /// Returns a mutable reference to the score director.
    pub fn score_director_mut(&mut self) -> &mut D {
        &mut self.score_director
    }

    /// Returns a reference to the working solution.
    pub fn working_solution(&self) -> &S {
        self.score_director.working_solution()
    }

    /// Returns a mutable reference to the working solution.
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

    /// Returns the best solution found so far.
    pub fn best_solution(&self) -> Option<&S> {
        self.best_solution.as_ref()
    }

    /// Returns the best score found so far.
    pub fn best_score(&self) -> Option<&S::Score> {
        self.best_score.as_ref()
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

            // Invoke callback if registered
            if let Some(ref solution) = self.best_solution {
                self.best_solution_callback.invoke(solution);
            }
        }
    }

    /// Forces an update of the best solution regardless of score comparison.
    pub fn set_best_solution(&mut self, solution: S, score: S::Score) {
        self.best_solution = Some(solution);
        self.best_score = Some(score);
    }

    /// Returns a reference to the RNG.
    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    /// Increments and returns the total step count.
    pub fn increment_step_count(&mut self) -> u64 {
        self.total_step_count += 1;
        self.stats.record_step();
        self.total_step_count
    }

    /// Returns the total step count.
    pub fn total_step_count(&self) -> u64 {
        self.total_step_count
    }

    /// Extracts the best solution, consuming this scope.
    pub fn take_best_solution(self) -> Option<S> {
        self.best_solution
    }

    /// Returns the best solution or the current working solution if no best was set.
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

    /// Checks if early termination was requested (external flag only).
    pub fn is_terminate_early(&self) -> bool {
        self.terminate
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
    }

    /// Sets the time limit for solving.
    pub fn set_time_limit(&mut self, limit: Duration) {
        self.time_limit = Some(limit);
    }

    /// Checks if construction heuristic should terminate.
    ///
    /// Construction phases must always complete — they build the initial solution.
    /// Step/move/score-calculation count limits are for local search phases only,
    /// so this method intentionally excludes those checks.
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

    /// Checks if solving should terminate (external flag, time limit, OR any registered limits).
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

    /// Returns a reference to the solver statistics.
    pub fn stats(&self) -> &SolverStats {
        &self.stats
    }

    /// Returns a mutable reference to the solver statistics.
    pub fn stats_mut(&mut self) -> &mut SolverStats {
        &mut self.stats
    }
}
