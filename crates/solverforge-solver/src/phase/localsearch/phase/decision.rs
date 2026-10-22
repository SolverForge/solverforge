//! Candidate generation records and in-order candidate consumption for one
//! local-search step.

use std::time::{Duration, Instant};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;
use tracing::trace;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, ResourceMoveCursor};
use crate::phase::localsearch::evaluation::{
    account_trial, record_evaluated_move, CandidateEvaluation, TrialOutcome,
};
use crate::phase::localsearch::{Acceptor, ForagerDecision, LocalSearchForager};
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken, CandidateTraceSource};

use super::candidates::CandidateLoopState;
use super::{candidate_selector_label, take_trace_token};

const PROGRESS_POLL_INTERVAL: u64 = 256;

/// A generated candidate waiting for its trial result.
pub(super) struct WindowSlot {
    pub(super) id: CandidateId,
    pub(super) selector_index: Option<usize>,
    pub(super) move_label: &'static str,
    pub(super) trace_token: Option<CandidateTracePullToken>,
}

/// Records one pulled candidate: trace pull, generation counters and
/// telemetry, and the periodic progress poll.
pub(super) fn record_generated_candidate<S, D, BestCb, M, C, Resources>(
    cursor: &C,
    candidate_id: CandidateId,
    generation_started: Instant,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    state: &mut CandidateLoopState,
) -> WindowSlot
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    let selector_index = cursor.selector_index(candidate_id);
    let mov = cursor
        .candidate(candidate_id)
        .expect("discovered candidate id must remain borrowable");
    let move_label = mov.telemetry_label();
    let selector_label = selector_index.map(|_| candidate_selector_label(&mov));
    let trace_token = step_scope.phase_scope_mut().record_candidate_pull(
        CandidateTraceSource::LocalSearch,
        selector_index,
        candidate_id.index(),
        None,
        &mov,
    );
    let generation_elapsed = generation_started.elapsed();
    state.generated_moves += 1;
    step_scope
        .phase_scope_mut()
        .record_move_kind_generated(move_label);
    if let Some(selector_index) = selector_index {
        step_scope
            .phase_scope_mut()
            .record_selector_generated_move_with_label(
                selector_index,
                selector_label.as_deref().unwrap_or("selector"),
                generation_elapsed,
            );
    } else {
        step_scope
            .phase_scope_mut()
            .record_generated_move(generation_elapsed);
    }
    if step_scope.progress_polling_required()
        && state.generated_moves.is_multiple_of(PROGRESS_POLL_INTERVAL)
    {
        step_scope.phase_scope_mut().report_progress_if_due();
    }
    WindowSlot {
        id: candidate_id,
        selector_index,
        move_label,
        trace_token,
    }
}

/// Feeds one trial result through accounting, the acceptor and the forager,
/// and releases the candidate when the step no longer needs it.
#[allow(clippy::too_many_arguments)]
pub(super) fn consume_candidate<S, D, BestCb, M, C, Resources, A, Fo>(
    cursor: &mut C,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    slot: &WindowSlot,
    outcome: TrialOutcome<S::Score>,
    elapsed: Duration,
    last_step_score: S::Score,
    acceptor: &mut A,
    forager: &mut Fo,
    requires_move_signatures: bool,
    state: &mut CandidateLoopState,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    let candidate_id = slot.id;
    let selector_index = slot.selector_index;
    let move_label = slot.move_label;
    let trace_token = slot.trace_token;
    state.evaluated_moves += 1;

    let mov = cursor
        .candidate(candidate_id)
        .expect("discovered candidate id must remain borrowable");
    let move_score = match account_trial(
        &mov,
        step_scope,
        last_step_score,
        selector_index,
        elapsed,
        outcome,
    ) {
        CandidateEvaluation::Scored(score) => {
            if let Some(token) = trace_token {
                step_scope
                    .phase_scope_mut()
                    .record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
            }
            step_scope
                .phase_scope_mut()
                .record_move_kind_evaluated(move_label, score.compare(&last_step_score));
            score
        }
        CandidateEvaluation::NotDoable => {
            record_rejected_trace(
                step_scope,
                trace_token,
                CandidateTraceDisposition::NotDoable,
            );
            assert!(cursor.release_candidate(candidate_id));
            return;
        }
        CandidateEvaluation::RejectedByHardImprovement(_) => {
            record_rejected_trace(
                step_scope,
                trace_token,
                CandidateTraceDisposition::RejectedByHardImprovement,
            );
            assert!(cursor.release_candidate(candidate_id));
            return;
        }
        CandidateEvaluation::RejectedByScoreImprovement(_) => {
            record_rejected_trace(
                step_scope,
                trace_token,
                CandidateTraceDisposition::RejectedByScoreImprovement,
            );
            assert!(cursor.release_candidate(candidate_id));
            return;
        }
    };
    let move_signature = if requires_move_signatures {
        match mov.borrowed_tabu_signature() {
            Some(borrowed) => Some(borrowed.clone()),
            None => Some(mov.tabu_signature(step_scope.score_director())),
        }
    } else {
        None
    };

    let accepted = acceptor.is_accepted(&last_step_score, &move_score, move_signature.as_ref());

    record_evaluated_move(step_scope, selector_index, elapsed);
    if accepted {
        step_scope
            .phase_scope_mut()
            .record_move_kind_accepted(move_label);
        state.accepted_labels.record(move_label);
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_accepted(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_accepted();
        }
        state.accepted_moves += 1;
    } else {
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::AcceptorRejected,
                );
        }
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_acceptor_rejected(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_acceptor_rejected();
        }
        step_scope
            .phase_scope_mut()
            .record_move_kind_acceptor_rejected(move_label, move_score.compare(&last_step_score));
    }

    trace!(
        event = "step",
        step = step_scope.step_index(),
        move_index = candidate_id.index(),
        score = %move_score,
        accepted = accepted,
    );

    if accepted {
        match forager.add_move_index(candidate_id, move_score) {
            ForagerDecision::Keep => {
                if let Some(token) = trace_token {
                    state.accepted_trace_tokens.push((candidate_id, token));
                }
            }
            ForagerDecision::Release => {
                if let Some(token) = trace_token {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                }
                assert!(cursor.release_candidate(candidate_id));
            }
            ForagerDecision::Replace(replaced_id) => {
                if let Some(token) = take_trace_token(&mut state.accepted_trace_tokens, replaced_id)
                {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                }
                if let Some(token) = trace_token {
                    state.accepted_trace_tokens.push((candidate_id, token));
                }
                assert!(cursor.release_candidate(replaced_id));
            }
        }
    } else {
        assert!(cursor.release_candidate(candidate_id));
    }
}

fn record_rejected_trace<S, D, BestCb>(
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    trace_token: Option<CandidateTracePullToken>,
    disposition: CandidateTraceDisposition,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    if let Some(token) = trace_token {
        let phase_scope = step_scope.phase_scope_mut();
        phase_scope.record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
        phase_scope.record_candidate_trace_disposition(token, disposition);
    }
}
