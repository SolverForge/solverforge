//! Bounded owner-restricted fallback behavior for regret construction.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::evaluation::{apply_insertion, eval_insertion, record_insertion_trial};
use super::precedence::{precedence_dispatch_order, solve_precedence_frontier_regret};
use super::{
    RegretAccess, OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET, OWNER_RESTRICTED_REGRET_TRIAL_BUDGET,
};
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::list_placement::OwnerRestriction;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

pub(super) fn owner_restricted_regret_trial_count(bucket_sizes: &[usize]) -> usize {
    bucket_sizes.iter().fold(0usize, |total, &len| {
        let owner_trials = len
            .saturating_mul(len.saturating_add(1))
            .saturating_mul(len.saturating_add(2))
            / 6;
        total.saturating_add(owner_trials)
    })
}

pub(super) fn owner_restricted_best_insertion_trial_count(bucket_sizes: &[usize]) -> usize {
    bucket_sizes.iter().fold(0usize, |total, &len| {
        total.saturating_add(len.saturating_mul(len.saturating_add(1)) / 2)
    })
}

fn owner_restricted_bucket_sizes<S, A>(
    access: &A,
    solution: &S,
    entries: &[SourceElement<A::Element>],
    entity_count: usize,
) -> Option<Vec<usize>>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
{
    let mut buckets = vec![0usize; entity_count];
    for entry in entries {
        let OwnerRestriction::Fixed(owner) =
            access.owner_restriction(solution, entity_count, &entry.element)
        else {
            return None;
        };
        if owner >= entity_count {
            return None;
        }
        buckets[owner] += 1;
    }
    Some(buckets)
}

pub(super) fn solve_oversized_owner_restricted<S, A, D, BestCb>(
    access: &A,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    source_index: &RuntimeListSourceIndex<A::Element>,
    entries: &[SourceElement<A::Element>],
    entity_count: usize,
    control_policy: StepControlPolicy,
) -> bool
where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let Some(buckets) = owner_restricted_bucket_sizes(
        access,
        phase_scope.score_director().working_solution(),
        entries,
        entity_count,
    ) else {
        return false;
    };
    if owner_restricted_regret_trial_count(&buckets) <= OWNER_RESTRICTED_REGRET_TRIAL_BUDGET {
        return false;
    }
    if solve_precedence_frontier_regret(
        access,
        phase_scope,
        source_index,
        entries,
        entity_count,
        control_policy,
    ) {
        return true;
    }
    if let Some(order) = precedence_dispatch_order(
        access,
        phase_scope.score_director().working_solution(),
        source_index,
        entries,
        entity_count,
    ) {
        apply_owner_ordered_append(access, phase_scope, entries, &order, control_policy);
        return true;
    }
    let estimated_work =
        owner_restricted_best_insertion_trial_count(&buckets).saturating_mul(entries.len());
    if estimated_work <= OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET {
        solve_owner_ordered_best_insertion(
            access,
            phase_scope,
            entries,
            entity_count,
            control_policy,
        );
    } else {
        solve_owner_ordered_append(
            access,
            phase_scope,
            source_index,
            entries,
            entity_count,
            control_policy,
        );
    }
    true
}

fn solve_owner_ordered_append<S, A, D, BestCb>(
    access: &A,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    source_index: &RuntimeListSourceIndex<A::Element>,
    entries: &[SourceElement<A::Element>],
    entity_count: usize,
    control_policy: StepControlPolicy,
) where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    if let Some(order) = precedence_dispatch_order(
        access,
        phase_scope.score_director().working_solution(),
        source_index,
        entries,
        entity_count,
    ) {
        apply_owner_ordered_append(access, phase_scope, entries, &order, control_policy);
        return;
    }
    let mut order = Vec::with_capacity(entries.len());
    for (entry_position, entry) in entries.iter().enumerate() {
        let restriction = access.owner_restriction(
            phase_scope.score_director().working_solution(),
            entity_count,
            &entry.element,
        );
        let OwnerRestriction::Fixed(owner) = restriction else {
            tracing::warn!("No valid owner found for owner-restricted regret fallback element");
            continue;
        };
        if owner >= entity_count {
            tracing::warn!("No valid owner found for owner-restricted regret fallback element");
            continue;
        }
        order.push((entry_position, owner));
    }
    apply_owner_ordered_append(access, phase_scope, entries, &order, control_policy);
}

fn apply_owner_ordered_append<S, A, D, BestCb>(
    access: &A,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    entries: &[SourceElement<A::Element>],
    order: &[(usize, usize)],
    control_policy: StepControlPolicy,
) where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let descriptor_index = access.descriptor_index();
    for &(entry_position, owner) in order {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            break;
        }
        let entry = &entries[entry_position];
        let insertion_position =
            access.list_len(phase_scope.score_director().working_solution(), owner);
        let trace_token = phase_scope.record_candidate_operation(
            CandidateTraceSource::ListRegretOwnerAppend,
            None,
            insertion_position,
            Some(CandidateTraceConstructionTarget {
                descriptor_index,
                entity_index: owner,
            }),
            descriptor_index,
            "list_owner_ordered_append",
            [
                entry.source_index as u64,
                owner as u64,
                insertion_position as u64,
            ],
        );
        if let Some(token) = trace_token {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
        }
        let mut step_scope = StepScope::new_with_control_policy(phase_scope, control_policy);
        step_scope.apply_committed_change(|director| {
            apply_insertion(access, entry, owner, insertion_position, director);
        });
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
        }
        let score = step_scope.calculate_score();
        step_scope.set_step_score(score);
        step_scope.complete();
    }
}

fn solve_owner_ordered_best_insertion<S, A, D, BestCb>(
    access: &A,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    entries: &[SourceElement<A::Element>],
    entity_count: usize,
    control_policy: StepControlPolicy,
) where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut order = Vec::with_capacity(entries.len());
    for (entry_position, entry) in entries.iter().enumerate() {
        let OwnerRestriction::Fixed(owner) = access.owner_restriction(
            phase_scope.score_director().working_solution(),
            entity_count,
            &entry.element,
        ) else {
            tracing::warn!("No valid owner found for owner-restricted regret fallback element");
            continue;
        };
        if owner >= entity_count {
            tracing::warn!("No valid owner found for owner-restricted regret fallback element");
            continue;
        }
        order.push((entry_position, owner));
    }
    apply_owner_ordered_best_insertion(access, phase_scope, entries, &order, control_policy);
}

fn apply_owner_ordered_best_insertion<S, A, D, BestCb>(
    access: &A,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    entries: &[SourceElement<A::Element>],
    order: &[(usize, usize)],
    control_policy: StepControlPolicy,
) where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    'entries: for &(entry_position, owner) in order {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            break;
        }
        let entry = &entries[entry_position];
        let len = access.list_len(phase_scope.score_director().working_solution(), owner);
        let mut best = None;
        for position in 0..=len {
            if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                if let Some((_, _, Some(token))) = best.take() {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                break 'entries;
            }
            let trace_token = record_insertion_trial(
                access,
                phase_scope,
                CandidateTraceSource::ListRegretInsertionTrial,
                position,
                entry,
                owner,
                position,
            );
            let score = eval_insertion(
                access,
                entry,
                owner,
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
            if best.is_none_or(|(_, best_score, _)| score > best_score) {
                if let Some((_, _, Some(token))) = best.take() {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                best = Some((position, score, trace_token));
            } else if let Some(token) = trace_token {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
            }
        }
        let Some((position, score, trace_token)) = best else {
            tracing::warn!("No valid owner-restricted insertion found for regret fallback element");
            continue;
        };
        let mut step_scope = StepScope::new_with_control_policy(phase_scope, control_policy);
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
        }
        step_scope.apply_committed_change(|director| {
            apply_insertion(access, entry, owner, position, director);
        });
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
        }
        step_scope.set_step_score(score);
        step_scope.complete();
    }
}
