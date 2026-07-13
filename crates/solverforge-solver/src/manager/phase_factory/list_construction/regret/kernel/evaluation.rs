//! Score-trial and selection primitives for canonical regret insertion.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::{candidate_entities, RegretAccess, RegretEvaluation, RegretValue};
use crate::builder::context::SourceElement;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTracePullToken,
    CandidateTraceSource,
};

pub(super) fn eval_insertion<S, A, D>(
    access: &A,
    entry: &SourceElement<A::Element>,
    entity_index: usize,
    position: usize,
    score_director: &mut D,
) -> S::Score
where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    let score_state = score_director.snapshot_score_state();
    score_director.before_variable_changed(descriptor_index, entity_index);
    access.insert_element(
        score_director.working_solution_mut(),
        entity_index,
        position,
        entry.element.clone(),
    );
    score_director.after_variable_changed(descriptor_index, entity_index);
    let score = score_director.calculate_score();
    score_director.before_variable_changed(descriptor_index, entity_index);
    access.remove_element(
        score_director.working_solution_mut(),
        entity_index,
        position,
    );
    score_director.after_variable_changed(descriptor_index, entity_index);
    score_director.restore_score_state(score_state);
    score
}

pub(super) fn apply_insertion<S, A, D>(
    access: &A,
    entry: &SourceElement<A::Element>,
    entity_index: usize,
    position: usize,
    score_director: &mut D,
) where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
{
    let descriptor_index = access.descriptor_index();
    score_director.before_variable_changed(descriptor_index, entity_index);
    access.insert_element(
        score_director.working_solution_mut(),
        entity_index,
        position,
        entry.element.clone(),
    );
    score_director.after_variable_changed(descriptor_index, entity_index);
}

pub(super) fn record_insertion_trial<S, A, D, BestCb>(
    access: &A,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    source: CandidateTraceSource,
    candidate_index: usize,
    entry: &SourceElement<A::Element>,
    entity_index: usize,
    insertion_index: usize,
) -> Option<CandidateTracePullToken>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let descriptor_index = access.descriptor_index();
    phase_scope.record_candidate_operation(
        source,
        None,
        candidate_index,
        Some(CandidateTraceConstructionTarget {
            descriptor_index,
            entity_index,
        }),
        descriptor_index,
        "list_insertion_trial",
        [
            entry.source_index as u64,
            entity_index as u64,
            insertion_index as u64,
        ],
    )
}

pub(super) fn evaluate_regret<S, A, D, BestCb>(
    access: &A,
    entry: &SourceElement<A::Element>,
    entity_count: usize,
    control_policy: StepControlPolicy,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) -> RegretEvaluation<(
    RegretValue<S::Score>,
    usize,
    usize,
    S::Score,
    Option<CandidateTracePullToken>,
)>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut best: Option<(usize, usize, S::Score, Option<CandidateTracePullToken>)> = None;
    let mut second_best: Option<S::Score> = None;
    let restriction = access.owner_restriction(
        phase_scope.score_director().working_solution(),
        entity_count,
        &entry.element,
    );
    for entity_index in candidate_entities(restriction, entity_count) {
        let len = access.list_len(
            phase_scope.score_director().working_solution(),
            entity_index,
        );
        for position in 0..=len {
            if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                if let Some((_, _, _, Some(token))) = best.take() {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                return RegretEvaluation::Interrupted;
            }
            let trace_token = record_insertion_trial(
                access,
                phase_scope,
                CandidateTraceSource::ListRegretInsertionTrial,
                position,
                entry,
                entity_index,
                position,
            );
            let score = eval_insertion(
                access,
                entry,
                entity_index,
                position,
                phase_scope.score_director_mut(),
            );
            phase_scope.record_score_calculation();
            if let Some(token) = trace_token {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::Evaluated,
                );
            }
            match best {
                None => best = Some((entity_index, position, score, trace_token)),
                Some((_, _, best_score, _)) if score > best_score => {
                    if let Some((_, _, _, Some(token))) = best.take() {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                    second_best = Some(best_score);
                    best = Some((entity_index, position, score, trace_token));
                }
                Some(_) => match second_best {
                    None => second_best = Some(score),
                    Some(existing_second) if score > existing_second => second_best = Some(score),
                    Some(_) => {}
                },
            }
            if !matches!(best, Some((best_entity, best_position, _, _)) if best_entity == entity_index && best_position == position)
            {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
            }
        }
    }

    let Some((entity_index, position, best_score, trace_token)) = best else {
        return RegretEvaluation::Complete(None);
    };
    let regret = second_best.map_or(RegretValue::Forced, |score| {
        RegretValue::Finite(best_score - score)
    });
    RegretEvaluation::Complete(Some((
        regret,
        entity_index,
        position,
        best_score,
        trace_token,
    )))
}

pub(super) fn evaluate_owner_regret<S, A, D, BestCb>(
    access: &A,
    entry: &SourceElement<A::Element>,
    owner_index: usize,
    control_policy: StepControlPolicy,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) -> RegretEvaluation<(
    RegretValue<S::Score>,
    usize,
    S::Score,
    Option<CandidateTracePullToken>,
)>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let len = access.list_len(phase_scope.score_director().working_solution(), owner_index);
    let mut best: Option<(usize, S::Score, Option<CandidateTracePullToken>)> = None;
    let mut second_best: Option<S::Score> = None;
    for position in 0..=len {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            if let Some((_, _, Some(token))) = best.take() {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
            }
            return RegretEvaluation::Interrupted;
        }
        let trace_token = record_insertion_trial(
            access,
            phase_scope,
            CandidateTraceSource::ListRegretInsertionTrial,
            position,
            entry,
            owner_index,
            position,
        );
        let score = eval_insertion(
            access,
            entry,
            owner_index,
            position,
            phase_scope.score_director_mut(),
        );
        phase_scope.record_score_calculation();
        if let Some(token) = trace_token {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
        }
        match best {
            None => best = Some((position, score, trace_token)),
            Some((_, best_score, _)) if score > best_score => {
                if let Some((_, _, Some(token))) = best.take() {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                second_best = Some(best_score);
                best = Some((position, score, trace_token));
            }
            Some(_) => match second_best {
                None => second_best = Some(score),
                Some(existing_second) if score > existing_second => second_best = Some(score),
                Some(_) => {}
            },
        }
        if !matches!(best, Some((best_position, _, _)) if best_position == position) {
            if let Some(token) = trace_token {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
            }
        }
    }

    let Some((position, best_score, trace_token)) = best else {
        return RegretEvaluation::Complete(None);
    };
    let regret = second_best.map_or(RegretValue::Forced, |score| {
        RegretValue::Finite(best_score - score)
    });
    RegretEvaluation::Complete(Some((regret, position, best_score, trace_token)))
}
