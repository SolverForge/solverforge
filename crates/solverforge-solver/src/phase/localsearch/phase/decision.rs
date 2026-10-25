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
    account_trial, record_evaluated_elapsed, CandidateEvaluation, TrialOutcome,
};
use crate::phase::localsearch::{Acceptor, ForagerDecision, LocalSearchForager};
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken, CandidateTraceSource};

use super::candidates::CandidateLoopState;
use super::{candidate_selector_label, take_trace_token};

pub(super) const PROGRESS_POLL_INTERVAL: u64 = 256;

/// A generated candidate waiting for its trial result.
pub(in crate::phase::localsearch) struct WindowSlot {
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
    // Moves that own a prebuilt signature lend it; only dynamic builders
    // produce one, and it lives for this decision alone.
    let built_signature;
    let move_signature = if requires_move_signatures {
        match mov.borrowed_tabu_signature() {
            Some(borrowed) => Some(borrowed),
            None => {
                built_signature = mov.tabu_signature(step_scope.score_director());
                Some(&built_signature)
            }
        }
    } else {
        None
    };

    let accepted = acceptor.is_accepted(&last_step_score, &move_score, move_signature);

    record_evaluated_elapsed(step_scope, selector_index, elapsed);
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

/// Ends the window at `index` because of an interruption: slot `index` gets
/// the sequential loop's `InterruptedBeforeEvaluation` record and stays live,
/// exactly as the sequential loop leaves it; later slots are discarded.
/// `evaluated` tells whether the window's trials already ran.
pub(super) fn interrupt_window_at<S, D, BestCb, M, C, Resources>(
    cursor: &mut C,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    window: &[WindowSlot],
    index: usize,
    evaluated: bool,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    let slot = &window[index];
    if let Some(token) = slot.trace_token {
        step_scope
            .phase_scope_mut()
            .record_candidate_trace_disposition(
                token,
                CandidateTraceDisposition::InterruptedBeforeEvaluation,
            );
    }
    if evaluated {
        step_scope.phase_scope_mut().record_move_speculative();
    }
    discard_window_from(cursor, step_scope, window, index + 1, evaluated);
}

/// Discards slots `from..` of a window that the step will not consume:
/// records `DiscardedSpeculative`, releases them, and counts them as
/// speculative work when their trials already ran.
pub(super) fn discard_window_from<S, D, BestCb, M, C, Resources>(
    cursor: &mut C,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    window: &[WindowSlot],
    from: usize,
    evaluated: bool,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    for slot in window.iter().skip(from) {
        if let Some(token) = slot.trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::DiscardedSpeculative,
                );
        }
        if evaluated {
            step_scope.phase_scope_mut().record_move_speculative();
        }
        assert!(cursor.release_candidate(slot.id));
    }
}

pub(super) fn record_rejected_trace<S, D, BestCb>(
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
