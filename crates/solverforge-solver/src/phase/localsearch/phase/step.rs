//! One complete local-search step around the shared candidate-stream kernel.

use std::time::Instant;

use rand::RngExt;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{MoveStreamContext, ResourceMoveCursor};
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_after_step, should_interrupt_before_candidate,
    StepInterrupt,
};
use crate::phase::localsearch::{Acceptor, LocalSearchForager, MoveCursorSource};
use crate::scope::{PendingControl, PhaseScope, ProgressCallback, StepScope};
use crate::stats::{AppliedMoveTelemetry, CandidateTraceDisposition};

use super::candidates::{evaluate_candidates, CandidateLoopState};
use super::take_trace_token;

pub(super) enum StepOutcome {
    Continue,
    Restart,
    Terminate,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_step<S, M, Source, A, Fo, D, BestCb>(
    move_source: &mut Source,
    resources: &mut Source::Resources,
    acceptor: &mut A,
    forager: &mut Fo,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    last_step_score: &mut S::Score,
) -> StepOutcome
where
    S: PlanningSolution,
    Source: MoveCursorSource<S, M>,
    M: Move<S>,
    A: Acceptor<S>,
    Fo: LocalSearchForager<S, M>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut step_scope = StepScope::new(phase_scope);

    /* Reset forager and acceptor for this step.
    Pass best and last-step scores so foragers that implement
    pick-early-on-improvement strategies know their reference targets.
    */
    let best_score = step_scope
        .phase_scope()
        .solver_scope()
        .best_score()
        .copied()
        .unwrap_or(*last_step_score);
    let step_index = step_scope.step_index();
    let step_seed = step_scope
        .phase_scope_mut()
        .solver_scope_mut()
        .rng()
        .random::<u64>();
    forager.step_started(best_score, *last_step_score, step_seed);
    acceptor.step_started();
    let requires_move_signatures = acceptor.requires_move_signatures();

    let interrupted = should_interrupt_before_candidate(&step_scope);
    let generation_started = Instant::now();
    let stream_context =
        MoveStreamContext::new(step_index, step_seed, forager.accepted_count_limit());
    let mut cursor =
        move_source.open_cursor(resources, step_scope.score_director(), stream_context);
    step_scope
        .phase_scope_mut()
        .record_generation_time(generation_started.elapsed());

    let CandidateLoopState {
        mut interrupted,
        accepted_moves,
        generated_moves,
        evaluated_moves,
        accepted_labels,
        mut accepted_trace_tokens,
    } = evaluate_candidates(
        &mut cursor,
        resources,
        &mut step_scope,
        *last_step_score,
        acceptor,
        forager,
        requires_move_signatures,
        CandidateLoopState::new(interrupted),
    );

    if !interrupted && should_interrupt_after_step(&step_scope) {
        interrupted = true;
    }

    let commit_interrupted_config_step = interrupted
        && accepted_moves > 0
        && matches!(
            step_scope.pending_control(),
            PendingControl::ConfigTerminationRequested
        );
    if interrupted && !commit_interrupted_config_step {
        match settle_search_interrupt(&mut step_scope) {
            StepInterrupt::Restart => {
                record_ignored_trace_tokens(&mut step_scope, &mut accepted_trace_tokens);
                return StepOutcome::Restart;
            }
            StepInterrupt::TerminatePhase => {
                record_ignored_trace_tokens(&mut step_scope, &mut accepted_trace_tokens);
                return StepOutcome::Terminate;
            }
        }
    }

    // The online forager retains only the selected candidate.
    let mut accepted_move_signature = None;
    if let Some((selected_index, selected_score)) = forager.pick_move_index() {
        let selected_trace_token = take_trace_token(&mut accepted_trace_tokens, selected_index);
        if let Some(token) = selected_trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
        }
        record_ignored_trace_tokens(&mut step_scope, &mut accepted_trace_tokens);
        let selector_index = cursor.selector_index(selected_index);
        let selected_move = cursor
            .candidate(selected_index)
            .expect("selected candidate id must remain borrowable until commit");
        let selected_move_label = selected_move.telemetry_label();
        if requires_move_signatures {
            accepted_move_signature =
                Some(selected_move.tabu_signature(step_scope.score_director()));
        }
        let previous_score = *last_step_score;
        step_scope.apply_committed_change(|score_director| {
            cursor.apply_owned_candidate(selected_index, score_director);
        });
        if let Some(token) = selected_trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
        }
        if let Some(selector_index) = selector_index {
            step_scope
                .phase_scope_mut()
                .record_selector_move_applied(selector_index);
        } else {
            step_scope.phase_scope_mut().record_move_applied();
        }
        step_scope.set_step_score(selected_score);
        let score_improvement = if previous_score.is_feasible() && selected_score > previous_score {
            selected_score.to_scalar() - previous_score.to_scalar()
        } else {
            0.0
        };
        step_scope
            .phase_scope_mut()
            .record_move_kind_applied(selected_move_label, score_improvement);
        if step_scope.phase_scope().can_record_applied_move_trace() {
            let score_before = previous_score.to_scalar();
            let score_after = selected_score.to_scalar();
            step_scope
                .phase_scope_mut()
                .record_applied_move_trace(AppliedMoveTelemetry {
                    step_index,
                    move_label: selected_move_label,
                    selected_candidate_index: selected_index.index(),
                    moves_generated_this_step: generated_moves,
                    moves_evaluated_this_step: evaluated_moves,
                    moves_accepted_this_step: accepted_moves,
                    moves_forager_ignored_this_step: accepted_moves.saturating_sub(1),
                    score_before,
                    score_after,
                    score_delta: score_after - score_before,
                    hard_feasible_before: previous_score.is_feasible(),
                    hard_feasible_after: selected_score.is_feasible(),
                });
        }

        *last_step_score = selected_score;
        step_scope.phase_scope_mut().update_best_solution();
        if accepted_moves > 1 {
            step_scope
                .phase_scope_mut()
                .record_moves_forager_ignored(accepted_moves - 1);
            accepted_labels.for_each_ignored_except_selected(
                Some(selected_move_label),
                |label, count| {
                    step_scope
                        .phase_scope_mut()
                        .record_move_kind_forager_ignored(label, count);
                },
            );
        }
    } else if accepted_moves > 0 {
        record_ignored_trace_tokens(&mut step_scope, &mut accepted_trace_tokens);
        step_scope
            .phase_scope_mut()
            .record_moves_forager_ignored(accepted_moves);
        accepted_labels.for_each_ignored_except_selected(None, |label, count| {
            step_scope
                .phase_scope_mut()
                .record_move_kind_forager_ignored(label, count);
        });
    }
    /* else: no accepted moves this step — that's fine, the acceptor
    history still needs to advance so Late Acceptance / SA / etc.
    can eventually escape the local optimum.
    */

    /* Always notify acceptor that step ended. For stateful acceptors
    (Late Acceptance, Simulated Annealing, Great Deluge, SCHC),
    the history must advance every step — even steps where no move
    was accepted — otherwise the acceptor state stalls.
    */
    acceptor.step_ended(last_step_score, accepted_move_signature.as_ref());

    step_scope.complete();
    StepOutcome::Continue
}

fn record_ignored_trace_tokens<S, D, BestCb>(
    step_scope: &mut StepScope<'_, '_, '_, S, D, BestCb>,
    tokens: &mut Vec<(
        crate::heuristic::selector::move_selector::CandidateId,
        crate::stats::CandidateTracePullToken,
    )>,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    for (_, token) in tokens.drain(..) {
        step_scope
            .phase_scope_mut()
            .record_candidate_trace_disposition(token, CandidateTraceDisposition::ForagerIgnored);
    }
}
