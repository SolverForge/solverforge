// Phase-level scope.

use std::cmp::Ordering;
use std::time::Duration;
use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::debug;

use super::solver::ProgressCallback;
use super::SolverScope;
use crate::stats::{
    whole_units_per_second, AppliedMoveTelemetry, CandidateTraceConstructionTarget,
    CandidateTraceDisposition, CandidateTracePullToken, CandidateTraceSource, PhaseStats,
};

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
    // Monotonic execution index used only by an enabled candidate trace.
    candidate_trace_phase_index: Option<usize>,
    // Score at the start of this phase.
    starting_score: Option<S::Score>,
    // Number of steps in this phase.
    step_count: u64,
    // When this phase started.
    start_time: Instant,
    // Solver-active elapsed at phase entry. The solver clock excludes pauses.
    solver_elapsed_at_start: Option<Duration>,
    // Phase statistics.
    stats: PhaseStats,
}

impl<'t, 'a, S: PlanningSolution, D: Director<S>, BestCb: ProgressCallback<S>>
    PhaseScope<'t, 'a, S, D, BestCb>
{
    pub fn new(solver_scope: &'a mut SolverScope<'t, S, D, BestCb>, phase_index: usize) -> Self {
        let candidate_trace_phase_index = solver_scope.begin_candidate_trace_phase();
        solver_scope.begin_phase_progress(phase_index, "Unknown", 0, 0);
        let starting_score = solver_scope.best_score().cloned();
        let solver_elapsed_at_start = solver_scope.elapsed();
        Self {
            solver_scope,
            phase_index,
            candidate_trace_phase_index,
            starting_score,
            step_count: 0,
            start_time: Instant::now(),
            solver_elapsed_at_start,
            stats: PhaseStats::new(phase_index, "Unknown"),
        }
    }

    pub fn with_phase_type(
        solver_scope: &'a mut SolverScope<'t, S, D, BestCb>,
        phase_index: usize,
        phase_type: &'static str,
    ) -> Self {
        let candidate_trace_phase_index = solver_scope.begin_candidate_trace_phase();
        solver_scope.begin_phase_progress(phase_index, phase_type, 0, 0);
        let starting_score = solver_scope.best_score().cloned();
        let solver_elapsed_at_start = solver_scope.elapsed();
        Self {
            solver_scope,
            phase_index,
            candidate_trace_phase_index,
            starting_score,
            step_count: 0,
            start_time: Instant::now(),
            solver_elapsed_at_start,
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
        match (self.solver_elapsed_at_start, self.solver_scope.elapsed()) {
            (Some(start), Some(now)) => now.saturating_sub(start),
            _ => self.start_time.elapsed(),
        }
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

    pub(crate) fn score_director_mut(&mut self) -> &mut D {
        self.solver_scope.score_director_mut()
    }

    pub fn mutate<T, F>(&mut self, mutate: F) -> T
    where
        F: FnOnce(&mut D) -> T,
    {
        self.solver_scope.mutate(mutate)
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

    pub fn report_progress(&self) {
        let phase = self.stats.snapshot_with_elapsed(self.elapsed());
        self.solver_scope.report_phase_progress(phase);
    }

    pub fn report_progress_if_due(&mut self) -> bool {
        if !BestCb::PUBLISHES_PROGRESS && !tracing::enabled!(tracing::Level::DEBUG) {
            return false;
        }
        let tick = self.solver_scope.take_phase_progress_tick(
            self.phase_index,
            self.stats.phase_type,
            self.stats.step_count,
            self.stats.moves_evaluated,
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
        debug!(
            event = "progress",
            phase = self.stats.phase_type,
            phase_index = self.phase_index,
            steps = self.stats.step_count,
            moves_generated = self.stats.moves_generated,
            moves_evaluated = self.stats.moves_evaluated,
            moves_accepted = self.stats.moves_accepted,
            moves_score_improving = self.stats.moves_score_improving(),
            moves_applied_improving = self.stats.moves_applied_improving(),
            score_calculations = self.stats.score_calculations,
            speed = whole_units_per_second(speed_count, tick.elapsed),
            acceptance_rate = format!("{:.1}%", self.stats.acceptance_rate() * 100.0),
            current_score = current_score,
            best_score = best_score,
        );
        self.report_progress();
        true
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

    pub fn record_selector_generated_move(&mut self, selector_index: usize, duration: Duration) {
        self.stats
            .record_selector_generated(selector_index, 1, duration);
        self.solver_scope
            .stats_mut()
            .record_selector_generated(selector_index, 1, duration);
    }

    pub fn record_selector_generated_move_with_label(
        &mut self,
        selector_index: usize,
        selector_label: impl Into<String>,
        duration: Duration,
    ) {
        let selector_label = selector_label.into();
        self.stats.record_selector_generated_with_label(
            selector_index,
            selector_label.clone(),
            1,
            duration,
        );
        self.solver_scope
            .stats_mut()
            .record_selector_generated_with_label(selector_index, selector_label, 1, duration);
    }

    pub fn record_generation_time(&mut self, duration: Duration) {
        self.stats.record_generation_time(duration);
        self.solver_scope
            .stats_mut()
            .record_generation_time(duration);
    }

    pub fn record_evaluated_move(&mut self, duration: Duration) {
        self.stats.record_evaluated_move(duration);
        self.solver_scope.record_evaluated_move(duration);
    }

    pub fn record_selector_evaluated_move(&mut self, selector_index: usize, duration: Duration) {
        self.stats
            .record_selector_evaluated(selector_index, duration);
        self.solver_scope
            .record_selector_evaluated(selector_index, duration);
    }

    pub fn record_move_accepted(&mut self) {
        self.stats.record_move_accepted();
        self.solver_scope.stats_mut().record_move_accepted();
    }

    pub fn record_move_applied(&mut self) {
        self.stats.record_move_applied();
        self.solver_scope.stats_mut().record_move_applied();
    }

    pub fn record_selector_move_accepted(&mut self, selector_index: usize) {
        self.stats.record_selector_accepted(selector_index);
        self.solver_scope
            .stats_mut()
            .record_selector_accepted(selector_index);
    }

    pub fn record_selector_move_applied(&mut self, selector_index: usize) {
        self.stats.record_selector_applied(selector_index);
        self.solver_scope
            .stats_mut()
            .record_selector_applied(selector_index);
    }

    pub fn record_move_not_doable(&mut self) {
        self.stats.record_move_not_doable();
        self.solver_scope.stats_mut().record_move_not_doable();
    }

    pub fn record_selector_move_not_doable(&mut self, selector_index: usize) {
        self.stats.record_selector_not_doable(selector_index);
        self.solver_scope
            .stats_mut()
            .record_selector_not_doable(selector_index);
    }

    pub fn record_move_acceptor_rejected(&mut self) {
        self.stats.record_move_acceptor_rejected();
        self.solver_scope
            .stats_mut()
            .record_move_acceptor_rejected();
    }

    pub fn record_selector_move_acceptor_rejected(&mut self, selector_index: usize) {
        self.stats.record_selector_acceptor_rejected(selector_index);
        self.solver_scope
            .stats_mut()
            .record_selector_acceptor_rejected(selector_index);
    }

    pub fn record_moves_forager_ignored(&mut self, count: u64) {
        self.stats.record_moves_forager_ignored(count);
        self.solver_scope
            .stats_mut()
            .record_moves_forager_ignored(count);
    }

    pub fn record_move_kind_generated(&mut self, move_label: &'static str) {
        self.stats.record_move_kind_generated(move_label);
        self.solver_scope
            .stats_mut()
            .record_move_kind_generated(move_label);
    }

    pub fn record_move_kind_evaluated(
        &mut self,
        move_label: &'static str,
        score_ordering: Ordering,
    ) {
        self.stats
            .record_move_kind_evaluated(move_label, score_ordering);
        self.solver_scope
            .stats_mut()
            .record_move_kind_evaluated(move_label, score_ordering);
    }

    pub fn record_move_kind_evaluated_unscored(&mut self, move_label: &'static str) {
        self.stats.record_move_kind_evaluated_unscored(move_label);
        self.solver_scope
            .stats_mut()
            .record_move_kind_evaluated_unscored(move_label);
    }

    pub fn record_move_kind_accepted(&mut self, move_label: &'static str) {
        self.stats.record_move_kind_accepted(move_label);
        self.solver_scope
            .stats_mut()
            .record_move_kind_accepted(move_label);
    }

    pub fn record_move_kind_applied(&mut self, move_label: &'static str, score_improvement: f64) {
        self.stats
            .record_move_kind_applied(move_label, score_improvement);
        self.solver_scope
            .stats_mut()
            .record_move_kind_applied(move_label, score_improvement);
    }

    pub fn record_move_kind_not_doable(&mut self, move_label: &'static str) {
        self.stats.record_move_kind_not_doable(move_label);
        self.solver_scope
            .stats_mut()
            .record_move_kind_not_doable(move_label);
    }

    pub fn record_move_kind_acceptor_rejected(
        &mut self,
        move_label: &'static str,
        score_ordering: Ordering,
    ) {
        self.stats
            .record_move_kind_acceptor_rejected(move_label, score_ordering);
        self.solver_scope
            .stats_mut()
            .record_move_kind_acceptor_rejected(move_label, score_ordering);
    }

    pub fn record_move_kind_forager_ignored(&mut self, move_label: &'static str, count: u64) {
        self.stats
            .record_move_kind_forager_ignored(move_label, count);
        self.solver_scope
            .stats_mut()
            .record_move_kind_forager_ignored(move_label, count);
    }

    pub fn record_applied_move_trace(&mut self, applied_move: AppliedMoveTelemetry) {
        self.stats.record_applied_move_trace(applied_move);
        self.solver_scope
            .stats_mut()
            .record_applied_move_trace(applied_move);
    }

    pub fn can_record_applied_move_trace(&self) -> bool {
        self.stats.can_record_applied_move_trace()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_candidate_pull<M>(
        &mut self,
        source: CandidateTraceSource,
        selector_index: Option<usize>,
        candidate_index: usize,
        construction_target: Option<CandidateTraceConstructionTarget>,
        candidate: &M,
    ) -> Option<CandidateTracePullToken>
    where
        M: crate::heuristic::r#move::Move<S>,
    {
        self.solver_scope.record_candidate_pull(
            source,
            self.candidate_trace_phase_index.unwrap_or(self.phase_index),
            self.stats.phase_type,
            self.step_count,
            selector_index,
            candidate_index,
            construction_target,
            candidate,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_candidate_operation<I, T>(
        &mut self,
        source: CandidateTraceSource,
        selector_index: Option<usize>,
        candidate_index: usize,
        construction_target: Option<CandidateTraceConstructionTarget>,
        descriptor_index: usize,
        operation: &'static str,
        components: I,
    ) -> Option<CandidateTracePullToken>
    where
        I: IntoIterator<Item = T>,
        T: Into<crate::stats::CandidateTraceCoordinate>,
    {
        self.solver_scope.record_candidate_operation(
            source,
            self.candidate_trace_phase_index.unwrap_or(self.phase_index),
            self.stats.phase_type,
            self.step_count,
            selector_index,
            candidate_index,
            construction_target,
            descriptor_index,
            operation,
            components,
        )
    }

    pub(crate) fn record_candidate_trace_disposition(
        &mut self,
        token: CandidateTracePullToken,
        disposition: CandidateTraceDisposition,
    ) {
        self.solver_scope
            .record_candidate_trace_disposition(token, disposition);
    }

    pub fn record_move_hard_improving(&mut self) {
        self.stats.record_move_hard_improving();
        self.solver_scope.stats_mut().record_move_hard_improving();
    }

    pub fn record_move_hard_neutral(&mut self) {
        self.stats.record_move_hard_neutral();
        self.solver_scope.stats_mut().record_move_hard_neutral();
    }

    pub fn record_move_hard_worse(&mut self) {
        self.stats.record_move_hard_worse();
        self.solver_scope.stats_mut().record_move_hard_worse();
    }

    pub fn record_score_calculation(&mut self) {
        self.stats.record_score_calculation();
        self.solver_scope.record_score_calculation();
    }

    pub fn record_construction_slot_assigned(&mut self) {
        self.stats.record_construction_slot_assigned();
        self.solver_scope
            .stats_mut()
            .record_construction_slot_assigned();
    }

    pub fn record_construction_slot_kept(&mut self) {
        self.stats.record_construction_slot_kept();
        self.solver_scope
            .stats_mut()
            .record_construction_slot_kept();
    }

    pub fn record_construction_slot_no_doable(&mut self) {
        self.stats.record_construction_slot_no_doable();
        self.solver_scope
            .stats_mut()
            .record_construction_slot_no_doable();
    }

    pub fn record_scalar_assignment_required_remaining(
        &mut self,
        group_name: &'static str,
        count: u64,
    ) {
        self.stats
            .record_scalar_assignment_required_remaining(count);
        self.solver_scope
            .stats_mut()
            .record_scalar_assignment_required_remaining(group_name, count);
    }
}
