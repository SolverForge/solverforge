use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::scope::{ProgressCallback, StepScope};
const VND_PROGRESS_POLL_INTERVAL: u64 = 0xFF;

pub(crate) struct VndProgress {
    moves_generated: u64,
    moves_evaluated: u64,
}

impl VndProgress {
    pub(crate) fn new() -> Self {
        Self {
            moves_generated: 0,
            moves_evaluated: 0,
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
        step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    ) where
        S: PlanningSolution,
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        if self.moves_evaluated & VND_PROGRESS_POLL_INTERVAL != 0 {
            return;
        }
        step_scope.phase_scope_mut().report_progress_if_due();
    }
}

pub(crate) fn candidate_selector_label<S, M>(mov: &MoveCandidateRef<'_, S, M>) -> String
where
    S: PlanningSolution,
    M: Move<S>,
{
    let move_label = mov.telemetry_label();
    if mov.variable_name() == "compound_scalar" || mov.variable_name() == "conflict_repair" {
        return format!("{}:{move_label}", mov.variable_name());
    }
    let mut label = None;
    mov.for_each_affected_entity(&mut |affected| {
        if label.is_none() {
            label = Some(affected.variable_name.to_string());
        }
    });
    label
        .map(|variable| format!("{variable}:{move_label}"))
        .unwrap_or_else(|| format!("move:{move_label}"))
}
