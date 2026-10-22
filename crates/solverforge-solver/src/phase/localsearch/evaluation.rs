use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveCandidateRef, MoveCandidateUndo};
use crate::phase::hard_delta::{hard_score_delta, HardScoreDelta};
use crate::pinning::move_changes_pinned;
use crate::scope::{ProgressCallback, StepScope};

pub(crate) enum CandidateEvaluation<Sc> {
    Scored(Sc),
    NotDoable,
    RejectedByHardImprovement(Sc),
    RejectedByScoreImprovement(Sc),
}

/* Director-only result of one trial transaction. Statistics are recorded by
`account_trial` on the thread that owns the step scope.
*/
pub(crate) enum TrialOutcome<Sc> {
    NotDoable,
    Scored(Sc),
}

#[inline]
pub(crate) fn evaluate_candidate<S, D, ProgressCb, M>(
    mov: &MoveCandidateRef<'_, S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    reference_score: S::Score,
    selector_index: Option<usize>,
    evaluation_started: Instant,
    journal_pool: &mut Vec<MoveCandidateUndo<M::Undo>>,
) -> CandidateEvaluation<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
{
    let outcome = run_trial(mov, step_scope.score_director_mut(), journal_pool);
    account_trial(
        mov,
        step_scope,
        reference_score,
        selector_index,
        evaluation_started.elapsed(),
        outcome,
    )
}

/* Runs the trial transaction on `director`: the pinned/doable check, then
apply, score, undo and restore. Touches nothing but the director and the
undo journal, so it can run on an evaluator replica.
*/
#[inline]
pub(crate) fn run_trial<S, D, M>(
    mov: &MoveCandidateRef<'_, S, M>,
    director: &mut D,
    journal_pool: &mut Vec<MoveCandidateUndo<M::Undo>>,
) -> TrialOutcome<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    M: Move<S>,
{
    if move_changes_pinned(mov, director) || !mov.is_doable_with_working_preview(director) {
        return TrialOutcome::NotDoable;
    }

    let score_state = director.snapshot_score_state();
    let undo = mov.do_move_with_scratch(director, journal_pool);
    let move_score = director.calculate_score();
    mov.undo_move_with_scratch(director, undo, journal_pool);
    director.restore_score_state(score_state);
    TrialOutcome::Scored(move_score)
}

/* Records the statistics and improvement-gate decisions for one trial
outcome, in the order the sequential evaluation loop has always used.
*/
#[inline]
pub(crate) fn account_trial<S, D, ProgressCb, M>(
    mov: &MoveCandidateRef<'_, S, M>,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    reference_score: S::Score,
    selector_index: Option<usize>,
    elapsed: Duration,
    outcome: TrialOutcome<S::Score>,
) -> CandidateEvaluation<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
{
    let move_score = match outcome {
        TrialOutcome::NotDoable => {
            let move_label = mov.telemetry_label();
            record_evaluated_move(step_scope, selector_index, elapsed);
            step_scope
                .phase_scope_mut()
                .record_move_kind_evaluated_unscored(move_label);
            if let Some(selector_index) = selector_index {
                step_scope
                    .phase_scope_mut()
                    .record_selector_move_not_doable(selector_index);
            } else {
                step_scope.phase_scope_mut().record_move_not_doable();
            }
            step_scope
                .phase_scope_mut()
                .record_move_kind_not_doable(move_label);
            return CandidateEvaluation::NotDoable;
        }
        TrialOutcome::Scored(move_score) => move_score,
    };

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
        let move_label = mov.telemetry_label();
        record_evaluated_move(step_scope, selector_index, elapsed);
        step_scope
            .phase_scope_mut()
            .record_move_kind_evaluated(move_label, move_score.compare(&reference_score));
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_acceptor_rejected(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_acceptor_rejected();
        }
        step_scope
            .phase_scope_mut()
            .record_move_kind_acceptor_rejected(move_label, move_score.compare(&reference_score));
        return CandidateEvaluation::RejectedByHardImprovement(move_score);
    }

    if mov.requires_score_improvement() && move_score <= reference_score {
        let move_label = mov.telemetry_label();
        record_evaluated_move(step_scope, selector_index, elapsed);
        step_scope
            .phase_scope_mut()
            .record_move_kind_evaluated(move_label, move_score.compare(&reference_score));
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_acceptor_rejected(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_acceptor_rejected();
        }
        step_scope
            .phase_scope_mut()
            .record_move_kind_acceptor_rejected(move_label, move_score.compare(&reference_score));
        return CandidateEvaluation::RejectedByScoreImprovement(move_score);
    }

    CandidateEvaluation::Scored(move_score)
}

#[inline]
pub(crate) fn record_evaluated_move<S, D, ProgressCb>(
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    selector_index: Option<usize>,
    elapsed: Duration,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    if let Some(selector_index) = selector_index {
        step_scope
            .phase_scope_mut()
            .record_selector_evaluated_move(selector_index, elapsed);
    } else {
        step_scope.phase_scope_mut().record_evaluated_move(elapsed);
    }
}
