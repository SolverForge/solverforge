//! Candidate streaming for one local-search step.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateUndo, ResourceMoveCursor,
};
use crate::phase::control::{
    should_interrupt_before_candidate, should_interrupt_before_evaluation, CONTROL_POLL_INTERVAL,
};
use crate::phase::localsearch::evaluation::run_trial;
use crate::phase::localsearch::{Acceptor, LocalSearchForager};
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken};

use super::decision::{consume_candidate, record_generated_candidate};
use super::StepMoveLabelCounts;

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
    // The control poll reads the wall clock; forwarding every candidate
    // iteration makes it measurable at scale (measured ~9% of wall before
    // batching). The step loop therefore polls once per batch window; a
    // stop stays latched, and per-evaluation interruption still runs on
    // its every-64-evaluations schedule below.
    // Retired undo journals are recycled across the step's trials so
    // arbitrary-long journals do not grow the heap per trial.
    let mut journal_pool: Vec<MoveCandidateUndo<M::Undo>> = Vec::new();
    let mut control_polls = 0usize;
    let mut control_latched = false;
    while !forager.is_quit_early() {
        if state.interrupted {
            break;
        }
        control_polls += 1;
        if !control_latched && control_polls.is_multiple_of(CONTROL_POLL_INTERVAL) {
            control_latched = should_interrupt_before_candidate(step_scope)
                || step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .should_terminate();
        }
        if control_latched {
            state.interrupted = true;
            break;
        }

        let generation_started = std::time::Instant::now();
        let Some(candidate_id) =
            cursor.next_candidate_with_resources(step_scope.score_director_mut(), resources)
        else {
            break;
        };
        let slot = record_generated_candidate(
            &*cursor,
            candidate_id,
            generation_started,
            step_scope,
            &mut state,
        );

        if should_interrupt_before_evaluation(step_scope)
            || step_scope
                .phase_scope_mut()
                .solver_scope_mut()
                .should_terminate()
        {
            if let Some(token) = slot.trace_token {
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

        let evaluation_started = std::time::Instant::now();
        let outcome = {
            let mov = cursor
                .candidate(slot.id)
                .expect("discovered candidate id must remain borrowable");
            run_trial(&mov, step_scope.score_director_mut(), &mut journal_pool)
        };
        consume_candidate(
            cursor,
            step_scope,
            &slot,
            outcome,
            evaluation_started.elapsed(),
            last_step_score,
            acceptor,
            forager,
            requires_move_signatures,
            &mut state,
        );
    }

    state
}
