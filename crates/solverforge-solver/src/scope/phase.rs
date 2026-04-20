// Phase-level scope.

use std::time::Duration;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::solver::ProgressCallback;
use super::SolverScope;
use crate::stats::PhaseStats;

/// Scope for a single phase of solving.
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag
/// * `'a` - Lifetime of the solver scope reference
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `BestCb` - The best-solution callback type
pub struct PhaseScope<'t, 'a, S: PlanningSolution, D: Director<S>, BestCb = ()> {
    // Reference to the parent solver scope.
    solver_scope: &'a mut SolverScope<'t, S, D, BestCb>,
    // Index of this phase (0-based).
    phase_index: usize,
    // Score at the start of this phase.
    starting_score: Option<S::Score>,
    // Number of steps in this phase.
    step_count: u64,
    // When this phase started.
    start_time: Instant,
    // Phase statistics.
    stats: PhaseStats,
}

impl<'t, 'a, S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>>
    PhaseScope<'t, 'a, S, D, BestCb>
{
    pub fn new(solver_scope: &'a mut SolverScope<'t, S, D, BestCb>, phase_index: usize) -> Self {
        let starting_score = solver_scope.best_score().cloned();
        Self {
            solver_scope,
            phase_index,
            starting_score,
            step_count: 0,
            start_time: Instant::now(),
            stats: PhaseStats::new(phase_index, "Unknown"),
        }
    }

    pub fn with_phase_type(
        solver_scope: &'a mut SolverScope<'t, S, D, BestCb>,
        phase_index: usize,
        phase_type: &'static str,
    ) -> Self {
        let starting_score = solver_scope.best_score().cloned();
        Self {
            solver_scope,
            phase_index,
            starting_score,
            step_count: 0,
            start_time: Instant::now(),
            stats: PhaseStats::new(phase_index, phase_type),
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

    /// Increments the phase step count.
    pub fn increment_step_count(&mut self) -> u64 {
        self.step_count += 1;
        self.stats.record_step();
        self.solver_scope.increment_step_count();
        self.step_count
    }

    pub fn solver_scope(&self) -> &SolverScope<'t, S, D, BestCb> {
        self.solver_scope
    }

    pub fn solver_scope_mut(&mut self) -> &mut SolverScope<'t, S, D, BestCb> {
        self.solver_scope
    }

    pub fn score_director(&self) -> &D {
        self.solver_scope.score_director()
    }

    pub fn score_director_mut(&mut self) -> &mut D {
        self.solver_scope.score_director_mut()
    }

    /// Calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.stats.record_score_calculation();
        self.solver_scope.calculate_score()
    }

    /// Updates best solution.
    pub fn update_best_solution(&mut self) {
        self.solver_scope.update_best_solution()
    }

    /// Publishes the current working solution when it ties the current best score.
    pub(crate) fn promote_current_solution_on_score_tie(&mut self) {
        self.solver_scope.promote_current_solution_on_score_tie()
    }

    pub fn stats(&self) -> &PhaseStats {
        &self.stats
    }

    pub fn stats_mut(&mut self) -> &mut PhaseStats {
        &mut self.stats
    }

    pub fn record_generated_batch(&mut self, count: u64, duration: Duration) {
        self.stats.record_generated_batch(count, duration);
        self.solver_scope
            .stats_mut()
            .record_generated_batch(count, duration);
    }

    pub fn record_generated_move(&mut self, duration: Duration) {
        self.record_generated_batch(1, duration);
    }

    pub fn record_generation_time(&mut self, duration: Duration) {
        self.stats.record_generation_time(duration);
        self.solver_scope
            .stats_mut()
            .record_generation_time(duration);
    }

    pub fn record_evaluated_move(&mut self, duration: Duration) {
        self.stats.record_evaluated_move(duration);
        self.solver_scope
            .stats_mut()
            .record_evaluated_move(duration);
    }

    pub fn record_move_accepted(&mut self) {
        self.stats.record_move_accepted();
        self.solver_scope.stats_mut().record_move_accepted();
    }

    pub fn record_score_calculation(&mut self) {
        self.stats.record_score_calculation();
        self.solver_scope.stats_mut().record_score_calculation();
    }
}
