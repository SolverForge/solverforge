use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::debug;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::whole_units_per_second;

const VND_PROGRESS_EVALUATION_INTERVAL: u64 = 0x1FFF;

pub(crate) struct VndProgress {
    moves_generated: u64,
    moves_evaluated: u64,
    last_progress_time: Instant,
    last_progress_evaluated: u64,
}

impl VndProgress {
    pub(crate) fn new() -> Self {
        Self {
            moves_generated: 0,
            moves_evaluated: 0,
            last_progress_time: Instant::now(),
            last_progress_evaluated: 0,
        }
    }

    pub(crate) fn moves_evaluated(&self) -> u64 {
        self.moves_evaluated
    }

    pub(crate) fn record_generated(&mut self) {
        self.moves_generated += 1;
    }

    pub(crate) fn record_evaluated(&mut self) {
        self.moves_evaluated += 1;
    }

    pub(crate) fn maybe_report<S, D, ProgressCb>(
        &mut self,
        step_scope: &StepScope<'_, '_, '_, S, D, ProgressCb>,
        current_score: &S::Score,
    ) where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        if self.moves_evaluated & VND_PROGRESS_EVALUATION_INTERVAL != 0 {
            return;
        }
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_progress_time);
        if elapsed.as_secs() < 1 {
            return;
        }
        let current_speed =
            whole_units_per_second(self.moves_evaluated - self.last_progress_evaluated, elapsed);
        let stats = step_scope.phase_scope().solver_scope().stats();
        debug!(
            event = "progress",
            phase = "Variable Neighborhood Descent",
            steps = step_scope.step_index(),
            moves_generated = self.moves_generated,
            moves_evaluated = self.moves_evaluated,
            moves_accepted = stats.moves_accepted,
            score_calculations = stats.score_calculations,
            speed = current_speed,
            acceptance_rate = format!("{:.1}%", stats.acceptance_rate() * 100.0),
            current_score = %current_score,
        );
        step_scope.phase_scope().solver_scope().report_progress();
        self.last_progress_time = now;
        self.last_progress_evaluated = self.moves_evaluated;
    }
}

pub(crate) fn candidate_selector_label<S, M>(mov: &MoveCandidateRef<'_, S, M>) -> String
where
    S: PlanningSolution,
    M: Move<S>,
{
    if mov.variable_name() == "compound_scalar" || mov.variable_name() == "conflict_repair" {
        return mov.variable_name().to_string();
    }
    let mut label = None;
    mov.for_each_affected_entity(&mut |affected| {
        if label.is_none() {
            label = Some(affected.variable_name.to_string());
        }
    });
    label.unwrap_or_else(|| "move".to_string())
}
