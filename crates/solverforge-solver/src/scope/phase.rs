//! Phase-level scope.

use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::SolverScope;

/// Scope for a single phase of solving.
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag
/// * `'a` - Lifetime of the solver scope reference
/// * `S` - The planning solution type
/// * `D` - The score director type
pub struct PhaseScope<'t, 'a, S: PlanningSolution, D: ScoreDirector<S>> {
    /// Reference to the parent solver scope.
    solver_scope: &'a mut SolverScope<'t, S, D>,
    /// Index of this phase (0-based).
    phase_index: usize,
    /// Score at the start of this phase.
    starting_score: Option<S::Score>,
    /// Number of steps in this phase.
    step_count: u64,
    /// When this phase started.
    start_time: Instant,
}

impl<'t, 'a, S: PlanningSolution, D: ScoreDirector<S>> PhaseScope<'t, 'a, S, D> {
    /// Creates a new phase scope.
    pub fn new(solver_scope: &'a mut SolverScope<'t, S, D>, phase_index: usize) -> Self {
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
    pub fn solver_scope(&self) -> &SolverScope<'t, S, D> {
        self.solver_scope
    }

    /// Returns a mutable reference to the solver scope.
    pub fn solver_scope_mut(&mut self) -> &mut SolverScope<'t, S, D> {
        self.solver_scope
    }

    /// Returns a reference to the score director.
    pub fn score_director(&self) -> &D {
        self.solver_scope.score_director()
    }

    /// Returns a mutable reference to the score director.
    pub fn score_director_mut(&mut self) -> &mut D {
        self.solver_scope.score_director_mut()
    }

    /// Calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.solver_scope.calculate_score()
    }

    /// Updates best solution.
    pub fn update_best_solution(&mut self) {
        self.solver_scope.update_best_solution()
    }
}
