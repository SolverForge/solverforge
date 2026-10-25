//! Candidate state for one local-search step and the parallel window loop.

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{CandidateId, ResourceMoveCursor};
use crate::phase::control::{
    should_interrupt_before_candidate, should_interrupt_before_evaluation, CONTROL_POLL_INTERVAL,
};
use crate::phase::localsearch::parallel::{
    evaluate_window, remaining_evaluation_budget, EvaluatorPool, MAX_PARALLEL_EVALUATION_WINDOW,
};
use crate::phase::localsearch::{Acceptor, LocalSearchForager};
use crate::scope::{ProgressCallback, StepScope};
use crate::stats::CandidateTracePullToken;

use super::decision::{
    consume_candidate, discard_window_from, interrupt_window_at, record_generated_candidate,
};
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

/// The parallel loop: produce a window serially on the main director,
/// evaluate it across the evaluator pool, and consume the results in
/// candidate order through the same checks and decisions.
#[allow(clippy::too_many_arguments)]
pub(super) fn evaluate_candidate_windows<S, D, BestCb, M, C, Resources, A, Fo>(
    cursor: &mut C,
    resources: &mut Resources,
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    last_step_score: S::Score,
    acceptor: &mut A,
    forager: &mut Fo,
    requires_move_signatures: bool,
    evaluators: &mut EvaluatorPool<S, D, M>,
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
    // stop stays latched, and per-evaluation interruption still runs before
    // every consumed candidate.
    // Retired undo journals are recycled across the step's trials so
    // arbitrary-long journals do not grow the heap per trial.
    //
    // Each pass produces a window of candidates serially on the main
    // director, evaluates it across the evaluator pool, and consumes the
    // results in candidate order through the sequential loop's checks.
    let mut journal_pool: Vec<M::Undo> = Vec::new();
    let mut control_polls = 0usize;
    let mut control_latched = false;
    let (mut window, mut results) = evaluators.take_window_buffers();
    let consumed_before = state.evaluated_moves;
    evaluators.begin_step();
    while !forager.is_quit_early() {
        if state.interrupted {
            break;
        }

        // Production: serial, canonical order, main director.
        let capacity = evaluators.window_capacity(remaining_evaluation_budget(
            step_scope.phase_scope().solver_scope(),
        ));
        window.clear();
        let mut exhausted = false;
        while window.len() < capacity {
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
                exhausted = true;
                break;
            };
            window.push(record_generated_candidate(
                &*cursor,
                candidate_id,
                generation_started,
                step_scope,
                &mut state,
            ));
        }
        if window.is_empty() {
            break;
        }

        // Dispatch: the sequential before-evaluation check for the first slot.
        if state.interrupted
            || should_interrupt_before_evaluation(step_scope)
            || step_scope
                .phase_scope_mut()
                .solver_scope_mut()
                .should_terminate()
        {
            interrupt_window_at(cursor, step_scope, &window, 0, false);
            state.interrupted = true;
            break;
        }

        // Evaluation. A window never exceeds the pool's ceiling, so the
        // borrowed candidates stay in inline storage.
        debug_assert!(window.len() <= MAX_PARALLEL_EVALUATION_WINDOW);
        let candidates: SmallVec<[_; MAX_PARALLEL_EVALUATION_WINDOW]> = window
            .iter()
            .map(|slot| {
                cursor
                    .candidate(slot.id)
                    .expect("window candidate must remain borrowable")
            })
            .collect();
        evaluate_window(
            &candidates,
            step_scope.score_director_mut(),
            &mut journal_pool,
            evaluators,
            &mut results,
        );
        // The borrowed candidates end here; consumption releases them.
        drop(candidates);

        // Consumption in candidate order.
        for index in 0..window.len() {
            if index > 0 {
                if forager.is_quit_early() {
                    discard_window_from(cursor, step_scope, &window, index, true);
                    break;
                }
                if should_interrupt_before_evaluation(step_scope)
                    || step_scope
                        .phase_scope_mut()
                        .solver_scope_mut()
                        .should_terminate()
                {
                    interrupt_window_at(cursor, step_scope, &window, index, true);
                    state.interrupted = true;
                    break;
                }
            }
            let (outcome, elapsed) = results[index]
                .take()
                .expect("every window slot has a trial result");
            consume_candidate(
                cursor,
                step_scope,
                &window[index],
                outcome,
                elapsed,
                last_step_score,
                acceptor,
                forager,
                requires_move_signatures,
                &mut state,
            );
        }
        if state.interrupted || exhausted {
            break;
        }
        evaluators.record_window(window.len());
    }

    evaluators.finish_step(state.evaluated_moves - consumed_before);
    evaluators.restore_window_buffers(window, results);
    state
}
