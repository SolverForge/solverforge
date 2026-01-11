//! Solver-level scope.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

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
pub struct SolverScope<'t, S: PlanningSolution, D: ScoreDirector<S>> {
    /// The score director managing the working solution.
    score_director: D,
    /// The best solution found so far.
    best_solution: Option<S>,
    /// The score of the best solution.
    best_score: Option<S::Score>,
    /// Random number generator for stochastic algorithms.
    rng: StdRng,
    /// When solving started.
    start_time: Option<Instant>,
    /// Total number of steps across all phases.
    total_step_count: u64,
    /// Flag for early termination requests.
    terminate: Option<&'t AtomicBool>,
    /// Solver statistics.
    stats: SolverStats,
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
    pub fn calculate_score(&mut self) -> S::Score {
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

    /// Checks if early termination was requested.
    pub fn is_terminate_early(&self) -> bool {
        self.terminate
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
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
