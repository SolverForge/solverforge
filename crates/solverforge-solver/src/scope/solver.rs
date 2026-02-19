//! Solver-level scope.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::SeedableRng;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::stats::SolverStats;

/// Top-level scope for the entire solving process.
///
/// Holds the working solution, score director, and tracks the best solution found.
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag reference
/// * `S` - The planning solution type
/// * `D` - The score director type
#[allow(clippy::type_complexity)]
pub struct SolverScope<'t, S: PlanningSolution, D: ScoreDirector<S>> {
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
    best_solution_callback: Option<Box<dyn Fn(&S) + Send + Sync + 't>>,
    // Additional termination check (set by Solver before running a phase).
    // Allows Termination trait implementations (step count, move count, etc.)
    // to be checked inside the phase step loop, not only between phases.
    termination_fn: Option<Box<dyn Fn(&SolverScope<'t, S, D>) -> bool + Send + Sync + 't>>,
    /// Optional maximum total step count for in-phase termination (T1).
    pub inphase_step_count_limit: Option<u64>,
    /// Optional maximum total move count for in-phase termination (T1).
    pub inphase_move_count_limit: Option<u64>,
    /// Optional maximum total score calculation count for in-phase termination (T1).
    pub inphase_score_calc_count_limit: Option<u64>,
}

impl<'t, S: PlanningSolution, D: ScoreDirector<S>> SolverScope<'t, S, D> {
    /// Creates a new solver scope with the given score director.
    pub fn new(score_director: D) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::from_os_rng(),
            start_time: None,
            total_step_count: 0,
            terminate: None,
            stats: SolverStats::default(),
            time_limit: None,
            best_solution_callback: None,
            termination_fn: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
        }
    }

    /// Creates a solver scope with a termination flag.
    pub fn with_terminate(score_director: D, terminate: Option<&'t AtomicBool>) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::from_os_rng(),
            start_time: None,
            total_step_count: 0,
            terminate,
            stats: SolverStats::default(),
            time_limit: None,
            best_solution_callback: None,
            termination_fn: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
        }
    }

    /// Creates a solver scope with a specific random seed.
    pub fn with_seed(score_director: D, seed: u64) -> Self {
        Self {
            score_director,
            best_solution: None,
            best_score: None,
            rng: StdRng::seed_from_u64(seed),
            start_time: None,
            total_step_count: 0,
            terminate: None,
            stats: SolverStats::default(),
            time_limit: None,
            best_solution_callback: None,
            termination_fn: None,
            inphase_step_count_limit: None,
            inphase_move_count_limit: None,
            inphase_score_calc_count_limit: None,
        }
    }

    /// Sets the best solution callback.
    ///
    /// The callback is invoked whenever the best solution improves during solving.
    pub fn with_best_solution_callback(
        mut self,
        callback: Box<dyn Fn(&S) + Send + Sync + 't>,
    ) -> Self {
        self.best_solution_callback = Some(callback);
        self
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
            if let Some(ref callback) = self.best_solution_callback {
                if let Some(ref solution) = self.best_solution {
                    callback(solution);
                }
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

    /// Registers a termination check function that is called inside phase step loops.
    ///
    /// This allows `Termination` trait implementations (e.g., step count, move count,
    /// score targets) to fire during a running phase, not only between phases.
    ///
    /// The solver sets this before calling `phase.solve()`.
    pub fn set_termination_fn(
        &mut self,
        f: Box<dyn Fn(&SolverScope<'t, S, D>) -> bool + Send + Sync + 't>,
    ) {
        self.termination_fn = Some(f);
    }

    /// Clears the registered termination function.
    pub fn clear_termination_fn(&mut self) {
        self.termination_fn = None;
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
        // Check registered termination function (covers any additional conditions)
        if let Some(ref f) = self.termination_fn {
            if f(self) {
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
