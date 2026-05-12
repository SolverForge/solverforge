use std::time::Instant;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::MoveCandidateRef;
use crate::phase::hard_delta::{hard_score_delta, HardScoreDelta};
use crate::scope::{ProgressCallback, StepScope};

pub(crate) enum CandidateEvaluation<Sc> {
    Scored(Sc),
    NotDoable,
    RejectedByHardImprovement,
}

#[inline]
pub(crate) fn evaluate_candidate<S, D, ProgressCb, M>(
    mov: &MoveCandidateRef<'_, S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    reference_score: S::Score,
    selector_index: Option<usize>,
    evaluation_started: Instant,
) -> CandidateEvaluation<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
{
    if !mov.is_doable(step_scope.score_director()) {
        record_evaluated_move(step_scope, selector_index, evaluation_started);
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_not_doable(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_not_doable();
        }
        return CandidateEvaluation::NotDoable;
    }

    let score_state = step_scope.score_director().snapshot_score_state();
    let undo = mov.do_move(step_scope.score_director_mut());
    let move_score = step_scope.score_director_mut().calculate_score();
    mov.undo_move(step_scope.score_director_mut(), undo);
    step_scope
        .score_director_mut()
        .restore_score_state(score_state);

    step_scope.phase_scope_mut().record_score_calculation();

    let hard_delta = hard_score_delta(reference_score, move_score);
    match hard_delta {
        Some(HardScoreDelta::Improving) => {
            step_scope.phase_scope_mut().record_move_hard_improving();
        }
        Some(HardScoreDelta::Neutral) => {
            step_scope.phase_scope_mut().record_move_hard_neutral();
        }
        Some(HardScoreDelta::Worse) => {
            step_scope.phase_scope_mut().record_move_hard_worse();
        }
        None => {}
    }

    if mov.requires_hard_improvement() && hard_delta != Some(HardScoreDelta::Improving) {
        record_evaluated_move(step_scope, selector_index, evaluation_started);
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_acceptor_rejected(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_acceptor_rejected();
        }
        return CandidateEvaluation::RejectedByHardImprovement;
    }

    CandidateEvaluation::Scored(move_score)
}

#[inline]
pub(crate) fn record_evaluated_move<S, D, ProgressCb>(
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    selector_index: Option<usize>,
    evaluation_started: Instant,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if let Some(selector_index) = selector_index {
        step_scope
            .phase_scope_mut()
            .record_selector_evaluated_move(selector_index, evaluation_started.elapsed());
    } else {
        step_scope
            .phase_scope_mut()
            .record_evaluated_move(evaluation_started.elapsed());
    }
}
