//! Candidate streaming for one local-search step.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;
use tracing::trace;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, ResourceMoveCursor};
use crate::phase::control::{
    should_interrupt_before_candidate, should_interrupt_before_evaluation,
};
use crate::phase::localsearch::evaluation::{
    evaluate_candidate, record_evaluated_move, CandidateEvaluation,
};
use crate::phase::localsearch::{Acceptor, ForagerDecision, LocalSearchForager};
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken, CandidateTraceSource};

use super::{candidate_selector_label, take_trace_token, StepMoveLabelCounts};

const PROGRESS_POLL_INTERVAL: u64 = 256;

pub(super) struct CandidateLoopState {
    pub(super) interrupted: bool,
    pub(super) accepted_moves: u64,
    pub(super) generated_moves: u64,
    pub(super) evaluated_moves: u64,
    pub(super) accepted_labels: StepMoveLabelCounts,
    pub(super) accepted_trace_tokens: Vec<(CandidateId, CandidateTracePullToken)>,
}

impl CandidateLoopState {
    pub(super) fn new(interrupted: bool) -> Self {
        Self {
            interrupted,
            accepted_moves: 0,
            generated_moves: 0,
            evaluated_moves: 0,
            accepted_labels: StepMoveLabelCounts::new(),
            accepted_trace_tokens: Vec::new(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate_candidates<S, D, BestCb, M, C, Resources, A, Fo>(
    cursor: &mut C,
    resources: &mut Resources,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    last_step_score: S::Score,
    acceptor: &mut A,
    forager: &mut Fo,
    requires_move_signatures: bool,
    mut state: CandidateLoopState,
) -> CandidateLoopState
where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
{
    while !forager.is_quit_early() {
        if state.interrupted || should_interrupt_before_candidate(step_scope) {
            state.interrupted = true;
            break;
        }
        if step_scope
            .phase_scope_mut()
            .solver_scope_mut()
            .should_terminate()
        {
            state.interrupted = true;
            break;
        }

        let generation_started = std::time::Instant::now();
        let Some(candidate_id) = cursor.next_candidate_with_resources(resources) else {
            break;
        };
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

        if should_interrupt_before_evaluation(step_scope)
            || step_scope
                .phase_scope_mut()
                .solver_scope_mut()
                .should_terminate()
        {
            if let Some(token) = trace_token {
                step_scope
                    .phase_scope_mut()
                    .record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::InterruptedBeforeEvaluation,
                    );
            }
            state.interrupted = true;
            break;
        }
        state.evaluated_moves += 1;

        let evaluation_started = std::time::Instant::now();
        let move_score = match evaluate_candidate(
            &mov,
            step_scope,
            last_step_score,
            selector_index,
            evaluation_started,
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
                continue;
            }
            CandidateEvaluation::RejectedByHardImprovement(_) => {
                record_rejected_trace(
                    step_scope,
                    trace_token,
                    CandidateTraceDisposition::RejectedByHardImprovement,
                );
                assert!(cursor.release_candidate(candidate_id));
                continue;
            }
            CandidateEvaluation::RejectedByScoreImprovement(_) => {
                record_rejected_trace(
                    step_scope,
                    trace_token,
                    CandidateTraceDisposition::RejectedByScoreImprovement,
                );
                assert!(cursor.release_candidate(candidate_id));
                continue;
            }
        };
        let move_signature = if requires_move_signatures {
            Some(mov.tabu_signature(step_scope.score_director()))
        } else {
            None
        };

        let accepted = acceptor.is_accepted(&last_step_score, &move_score, move_signature.as_ref());

        record_evaluated_move(step_scope, selector_index, evaluation_started);
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
                .record_move_kind_acceptor_rejected(
                    move_label,
                    move_score.compare(&last_step_score),
                );
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
                    if let Some(token) =
                        take_trace_token(&mut state.accepted_trace_tokens, replaced_id)
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

    state
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
