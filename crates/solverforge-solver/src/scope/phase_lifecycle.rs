//! Phase scope lifecycle: opening a phase, its parent scopes, scoring and
//! progress reporting.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::debug;

use super::PhaseScope;
use crate::scope::solver::ProgressCallback;
use crate::scope::SolverScope;
use crate::stats::{whole_units_per_second, PhaseStats};

impl<'t, 'a, S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>>
    PhaseScope<'t, 'a, S, D, BestCb>
{
    /// Opens phase `phase_index` with the phase type `Unknown`; see
    /// [`Self::with_phase_type`].
    pub fn new(solver_scope: &'a mut SolverScope<'t, S, D, BestCb>, phase_index: usize) -> Self {
        Self::with_phase_type(solver_scope, phase_index, "Unknown")
    }

    /// Opens phase `phase_index` of type `phase_type`: starts the phase's
    /// statistics record and progress pulse and remembers the current best
    /// score as the starting score. The record closes when the scope drops.
    pub fn with_phase_type(
        solver_scope: &'a mut SolverScope<'t, S, D, BestCb>,
        phase_index: usize,
        phase_type: &'static str,
    ) -> Self {
        let candidate_trace_phase_index = solver_scope.begin_candidate_trace_phase();
        solver_scope.begin_phase_progress(phase_index, phase_type, 0, 0);
        solver_scope.stats_mut().begin_phase(phase_type);
        let starting_score = solver_scope.best_score().cloned();
        Self {
            solver_scope,
            phase_index,
            candidate_trace_phase_index,
            starting_score,
            step_count: 0,
            phase_type,
        }
    }

    /// Index of this phase in the solve (0-based).
    pub fn phase_index(&self) -> usize {
        self.phase_index
    }

    /// Best score when the phase started; `None` if no best solution existed.
    pub fn starting_score(&self) -> Option<&S::Score> {
        self.starting_score.as_ref()
    }

    /// Solver-active time this phase has spent; the solver clock excludes
    /// pauses.
    pub fn elapsed(&self) -> std::time::Duration {
        self.solver_scope
            .stats()
            .phase_elapsed()
            .expect("a phase scope keeps its phase record open")
    }

    /// Steps completed in this phase.
    pub fn step_count(&self) -> u64 {
        self.step_count
    }

    /// Increments the phase step count.
    pub fn increment_step_count(&mut self) -> u64 {
        self.step_count += 1;
        self.solver_scope.increment_step_count();
        self.step_count
    }

    /// The parent solver scope.
    pub fn solver_scope(&self) -> &SolverScope<'t, S, D, BestCb> {
        self.solver_scope
    }

    /// The parent solver scope, mutably.
    pub fn solver_scope_mut(&mut self) -> &mut SolverScope<'t, S, D, BestCb> {
        self.solver_scope
    }

    /// The score director holding the working solution.
    pub fn score_director(&self) -> &D {
        self.solver_scope.score_director()
    }

    pub(crate) fn score_director_mut(&mut self) -> &mut D {
        self.solver_scope.score_director_mut()
    }

    /// Applies a committed change to the working solution through the score
    /// director and returns the closure's result. The cached current score is
    /// cleared and the working-solution revision advances.
    pub fn mutate<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.solver_scope.mutate(mutate)
    }

    /// Calculates the current score.
    pub fn calculate_score(&mut self) -> S::Score {
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

    /// Statistics of this phase. They live in the solver's statistics, which
    /// open this phase's record for the scope's lifetime.
    pub fn stats(&self) -> &PhaseStats {
        self.solver_scope
            .stats()
            .phase()
            .expect("a phase scope keeps its phase record open")
    }

    /// Statistics of this phase, mutably.
    pub fn stats_mut(&mut self) -> &mut PhaseStats {
        self.solver_scope
            .stats_mut()
            .phase_mut()
            .expect("a phase scope keeps its phase record open")
    }

    /// Publishes a progress report through the solver's progress callback
    /// (without the applied-move trace).
    pub fn report_progress(&self) {
        self.solver_scope.report_phase_progress();
    }

    /// Logs a `progress` debug event and publishes a progress report when the
    /// progress interval has elapsed. Returns whether a report was made; always
    /// `false` when neither a progress callback nor debug logging is active.
    pub fn report_progress_if_due(&mut self) -> bool {
        if !BestCb::PUBLISHES_PROGRESS && !tracing::enabled!(tracing::Level::DEBUG) {
            return false;
        }
        let (step_count, moves_evaluated) = {
            let stats = self.stats();
            (stats.step_count, stats.moves_evaluated)
        };
        let tick = self.solver_scope.take_phase_progress_tick(
            self.phase_index,
            self.phase_type,
            step_count,
            moves_evaluated,
        );
        let Some(tick) = tick else {
            return false;
        };
        let speed_count = if tick.move_delta > 0 {
            tick.move_delta
        } else {
            tick.step_delta
        };
        let current_score = self.solver_scope.current_score();
        let published_best_score = self.solver_scope.best_score();
        let reported_best_score = match (current_score, published_best_score) {
            (Some(current), Some(best)) if current > best => Some(current),
            (_, Some(best)) => Some(best),
            (Some(current), None) => Some(current),
            (None, None) => None,
        };
        let current_score = current_score
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string());
        let best_score = reported_best_score
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string());
        let stats = self.stats();
        debug!(
            event = "progress",
            phase = self.phase_type,
            phase_index = self.phase_index,
            steps = stats.step_count,
            moves_generated = stats.moves_generated,
            moves_evaluated = stats.moves_evaluated,
            moves_accepted = stats.moves_accepted,
            moves_score_improving = stats.moves_score_improving(),
            moves_applied_improving = stats.moves_applied_improving(),
            score_calculations = stats.score_calculations,
            speed = whole_units_per_second(speed_count, tick.elapsed),
            acceptance_rate = format!("{:.1}%", stats.acceptance_rate() * 100.0),
            current_score = current_score,
            best_score = best_score,
        );
        self.report_progress();
        true
    }
}
