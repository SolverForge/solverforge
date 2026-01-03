//! Phase-level scope.

use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::SolverScope;

/// Scope for a single phase of solving.
pub struct PhaseScope<'a, S: PlanningSolution> {
    /// Reference to the parent solver scope.
    solver_scope: &'a mut SolverScope<S>,
    /// Index of this phase (0-based).
    phase_index: usize,
    /// Score at the start of this phase.
    starting_score: Option<S::Score>,
    /// Number of steps in this phase.
    step_count: u64,
    /// When this phase started.
    start_time: Instant,
}

impl<'a, S: PlanningSolution> PhaseScope<'a, S> {
    /// Creates a new phase scope.
    pub fn new(solver_scope: &'a mut SolverScope<S>, phase_index: usize) -> Self {
        let starting_score = solver_scope.best_score().cloned();
        Self {
            solver_scope,
            phase_index,
            starting_score,
            step_count: 0,
            start_time: Instant::now(),
        }
    }

    /// Returns the phase index.
    pub fn phase_index(&self) -> usize {
        self.phase_index
    }

    /// Returns the starting score for this phase.
    pub fn starting_score(&self) -> Option<&S::Score> {
        self.starting_score.as_ref()
    }

    /// Returns the elapsed time for this phase.
    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    /// Returns the step count for this phase.
    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Increments the phase step count.
    pub fn increment_step_count(&mut self) -> u64 {
        self.step_count += 1;
        self.solver_scope.increment_step_count();
        self.step_count
    }

    /// Returns a reference to the solver scope.
    pub fn solver_scope(&self) -> &SolverScope<S> {
        self.solver_scope
    }

    /// Returns a mutable reference to the solver scope.
    pub fn solver_scope_mut(&mut self) -> &mut SolverScope<S> {
        self.solver_scope
    }

    /// Convenience: returns the score director.
    pub fn score_director(&self) -> &dyn ScoreDirector<S> {
        self.solver_scope.score_director()
    }

    /// Convenience: returns a mutable score director.
    pub fn score_director_mut(&mut self) -> &mut dyn ScoreDirector<S> {
        self.solver_scope.score_director_mut()
    }

    /// Convenience: calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.solver_scope.calculate_score()
    }

    /// Convenience: updates best solution.
    pub fn update_best_solution(&mut self) {
        self.solver_scope.update_best_solution()
    }
}
