//! Phase-level scope.

use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::SolverScope;

/// Scope for a single phase of solving.
pub struct PhaseScope<'a, S: PlanningSolution, D: ScoreDirector<S>> {
    solver_scope: &'a mut SolverScope<S, D>,
    phase_index: usize,
    starting_score: Option<S::Score>,
    step_count: u64,
    start_time: Instant,
}

impl<'a, S: PlanningSolution, D: ScoreDirector<S>> PhaseScope<'a, S, D> {
    pub fn new(solver_scope: &'a mut SolverScope<S, D>, phase_index: usize) -> Self {
        let starting_score = solver_scope.best_score().cloned();
        Self {
            solver_scope,
            phase_index,
            starting_score,
            step_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn phase_index(&self) -> usize {
        self.phase_index
    }

    pub fn starting_score(&self) -> Option<&S::Score> {
        self.starting_score.as_ref()
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    pub fn increment_step_count(&mut self) -> u64 {
        self.step_count += 1;
        self.solver_scope.increment_step_count();
        self.step_count
    }

    pub fn solver_scope(&self) -> &SolverScope<S, D> {
        self.solver_scope
    }

    pub fn solver_scope_mut(&mut self) -> &mut SolverScope<S, D> {
        self.solver_scope
    }

    pub fn score_director(&self) -> &D {
        self.solver_scope.score_director()
    }

    pub fn score_director_mut(&mut self) -> &mut D {
        self.solver_scope.score_director_mut()
    }

    pub fn calculate_score(&mut self) -> S::Score {
        self.solver_scope.calculate_score()
    }

    pub fn update_best_solution(&mut self) {
        self.solver_scope.update_best_solution()
    }
}
