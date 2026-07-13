use std::fmt::Debug;
use std::time::Instant;

use rand::RngExt;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;
use tracing::info;

use crate::heuristic::r#move::Move;
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveStreamContext, ResourceMoveCursor,
};
use crate::phase::control::{
    settle_search_interrupt, should_interrupt_after_step, should_interrupt_before_candidate,
    should_interrupt_before_evaluation, StepInterrupt,
};
use crate::phase::localsearch::evaluation::{
    evaluate_candidate, record_evaluated_move, CandidateEvaluation,
};
use crate::phase::localsearch::vnd::telemetry::{candidate_selector_label, VndProgress};
use crate::phase::localsearch::MoveCursorSource;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};
use crate::stats::{
    format_duration, whole_units_per_second, CandidateTraceDisposition, CandidateTracePullToken,
    CandidateTraceSource,
};

/// Executes the one VND loop while lending the caller-owned resource only at
/// cursor opening and candidate pull boundaries. The compiled runner passes
/// its one solve-owned provider resource directly.
pub(crate) fn solve_vnd_with_resources<S, D, ProgressCb, M, Source>(
    neighborhoods: &mut [Source],
    resources: &mut Source::Resources,
    step_limit: Option<u64>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    Source: MoveCursorSource<S, M> + Debug + Send,
{
    let phase_name = "Variable Neighborhood Descent";
    let mut phase_scope = PhaseScope::with_phase_type(solver_scope, 0, phase_name);
    let phase_index = phase_scope.phase_index();
    let mut current_score = phase_scope.calculate_score();
    let mut progress = VndProgress::new();
    let mut k = 0usize;

    info!(
        event = "phase_start",
        phase = phase_name,
        phase_index = phase_index,
        score = %current_score,
    );
    phase_scope.report_progress();

    while k < neighborhoods.len() {
        if phase_scope.solver_scope_mut().should_terminate() {
            break;
        }

        if let Some(limit) = step_limit {
            if phase_scope.step_count() >= limit {
                break;
            }
        }

        let mut step_scope = StepScope::new(&mut phase_scope);
        let stream_context = MoveStreamContext::new(
            step_scope.step_index(),
            step_scope
                .phase_scope_mut()
                .solver_scope_mut()
                .rng()
                .random::<u64>(),
            None,
        );
        let mut cursor =
            neighborhoods[k].open_cursor(resources, step_scope.score_director(), stream_context);

        match find_best_improving_move(
            &mut cursor,
            resources,
            &mut step_scope,
            &current_score,
            &mut progress,
        ) {
            MoveSearchResult::Found(
                selected_index,
                selected_score,
                selector_index,
                selected_trace_token,
            ) => {
                if let Some(token) = selected_trace_token {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Selected,
                        );
                }
                cursor
                    .candidate(selected_index)
                    .expect("selected VND candidate id must remain borrowable until commit");
                step_scope.apply_committed_change(|score_director| {
                    cursor.apply_owned_candidate(selected_index, score_director);
                });
                if let Some(token) = selected_trace_token {
                    step_scope
                        .phase_scope_mut()
                        .record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Applied,
                        );
                }
                if let Some(selector_index) = selector_index {
                    step_scope
                        .phase_scope_mut()
                        .record_selector_move_accepted(selector_index);
                    step_scope
                        .phase_scope_mut()
                        .record_selector_move_applied(selector_index);
                } else {
                    step_scope.phase_scope_mut().record_move_accepted();
                    step_scope.phase_scope_mut().record_move_applied();
                }
                step_scope.set_step_score(selected_score);
                current_score = selected_score;
                step_scope.phase_scope_mut().update_best_solution();
                step_scope.complete();
                k = 0;
            }
            MoveSearchResult::NotFound => {
                step_scope.complete();
                k += 1;
            }
            MoveSearchResult::Interrupted => match settle_search_interrupt(&mut step_scope) {
                StepInterrupt::Restart => continue,
                StepInterrupt::TerminatePhase => break,
            },
        }
    }

    phase_scope.report_progress();
    let duration = phase_scope.elapsed();
    let steps = phase_scope.step_count();
    let speed = whole_units_per_second(progress.moves_evaluated(), duration);
    let stats = phase_scope.stats();
    info!(
        event = "phase_end",
        phase = phase_name,
        phase_index = phase_index,
        duration = %format_duration(duration),
        steps = steps,
        moves_generated = stats.moves_generated,
        moves_evaluated = stats.moves_evaluated,
        moves_accepted = stats.moves_accepted,
        moves_score_improving = stats.moves_score_improving(),
        moves_applied_improving = stats.moves_applied_improving(),
        score_calculations = stats.score_calculations,
        generation_time = %format_duration(stats.generation_time()),
        evaluation_time = %format_duration(stats.evaluation_time()),
        speed = speed,
        score = %current_score,
    );
}

enum MoveSearchResult<Sc> {
    Found(
        CandidateId,
        Sc,
        Option<usize>,
        Option<CandidateTracePullToken>,
    ),
    NotFound,
    Interrupted,
}

#[allow(clippy::drop_non_drop)]
fn find_best_improving_move<S, D, ProgressCb, M, C, Resources>(
    cursor: &mut C,
    resources: &mut Resources,
    step_scope: &mut StepScope<'_, '_, '_, S, D, ProgressCb>,
    current_score: &S::Score,
    progress: &mut VndProgress,
) -> MoveSearchResult<S::Score>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    M: Move<S>,
    C: ResourceMoveCursor<S, M, Resources>,
{
    let mut best: Option<(CandidateId, S::Score, Option<CandidateTracePullToken>)> = None;

    loop {
        if should_interrupt_before_candidate(step_scope) {
            if let Some((_, _, Some(token))) = best.take() {
                step_scope
                    .phase_scope_mut()
                    .record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
            }
            return MoveSearchResult::Interrupted;
        }
        let generation_started = Instant::now();
        let Some(candidate_index) = cursor.next_candidate_with_resources(resources) else {
            break;
        };
        let generation_elapsed = generation_started.elapsed();
        let mov = cursor
            .candidate(candidate_index)
            .expect("discovered candidate id must remain borrowable");
        let selector_index = cursor.selector_index(candidate_index);
        let selector_label = selector_index.map(|_| candidate_selector_label(&mov));
        let trace_token = step_scope.phase_scope_mut().record_candidate_pull(
            CandidateTraceSource::VariableNeighborhoodDescent,
            selector_index,
            candidate_index.index(),
            None,
            &mov,
        );
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
        progress.record_generated();

        if should_interrupt_before_evaluation(step_scope) {
            if let Some(token) = trace_token {
                step_scope
                    .phase_scope_mut()
                    .record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::InterruptedBeforeEvaluation,
                    );
            }
            if let Some((_, _, Some(token))) = best.take() {
                step_scope
                    .phase_scope_mut()
                    .record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
            }
            return MoveSearchResult::Interrupted;
        }
        let evaluation_started = Instant::now();
        let move_score = match evaluate_candidate(
            &mov,
            step_scope,
            *current_score,
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
                score
            }
            CandidateEvaluation::NotDoable => {
                if let Some(token) = trace_token {
                    let phase_scope = step_scope.phase_scope_mut();
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::NotDoable,
                    );
                }
                progress.record_evaluated();
                progress.maybe_report(step_scope);
                drop(mov);
                assert!(cursor.release_candidate(candidate_index));
                continue;
            }
            CandidateEvaluation::RejectedByHardImprovement(_) => {
                if let Some(token) = trace_token {
                    let phase_scope = step_scope.phase_scope_mut();
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::RejectedByHardImprovement,
                    );
                }
                progress.record_evaluated();
                progress.maybe_report(step_scope);
                drop(mov);
                assert!(cursor.release_candidate(candidate_index));
                continue;
            }
            CandidateEvaluation::RejectedByScoreImprovement(_) => {
                if let Some(token) = trace_token {
                    let phase_scope = step_scope.phase_scope_mut();
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::RejectedByScoreImprovement,
                    );
                }
                progress.record_evaluated();
                progress.maybe_report(step_scope);
                drop(mov);
                assert!(cursor.release_candidate(candidate_index));
                continue;
            }
        };

        record_evaluated_move(step_scope, selector_index, evaluation_started);
        progress.record_evaluated();
        progress.maybe_report(step_scope);

        if move_score > *current_score {
            match &best {
                Some((_, best_score, _)) if move_score > *best_score => {
                    let (previous_best, _, previous_token) = best
                        .take()
                        .expect("VND best candidate must exist before replacement");
                    drop(mov);
                    assert!(cursor.release_candidate(previous_best));
                    if let Some(token) = previous_token {
                        step_scope
                            .phase_scope_mut()
                            .record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::ForagerIgnored,
                            );
                    }
                    best = Some((candidate_index, move_score, trace_token));
                }
                None => best = Some((candidate_index, move_score, trace_token)),
                _ => {
                    drop(mov);
                    assert!(cursor.release_candidate(candidate_index));
                    if let Some(token) = trace_token {
                        step_scope
                            .phase_scope_mut()
                            .record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::ForagerIgnored,
                            );
                    }
                }
            }
        } else {
            drop(mov);
            assert!(cursor.release_candidate(candidate_index));
            if let Some(token) = trace_token {
                step_scope
                    .phase_scope_mut()
                    .record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
            }
        }
    }

    if should_interrupt_after_step(step_scope) {
        if let Some((_, _, Some(token))) = best.take() {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
        }
        return MoveSearchResult::Interrupted;
    }

    match best {
        Some((index, score, token)) => {
            let selector_index = cursor.selector_index(index);
            MoveSearchResult::Found(index, score, selector_index, token)
        }
        None => MoveSearchResult::NotFound,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
