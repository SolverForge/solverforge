//! Main canonical regret-insertion loop.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::evaluation::{apply_insertion, evaluate_regret};
use super::{regret_choice_is_better_with_downstream, RegretAccess, RegretEvaluation, RegretValue};
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepControlPolicy, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken};

use super::fallback::solve_oversized_owner_restricted;
use super::precedence::precedence_downstream_by_source;

/// Runs the one canonical regret-insertion algorithm over a frozen source.
///
/// The caller must bind source identity before phase execution.  The kernel
/// retains source positions through every candidate, precedence, fallback,
/// and trace decision and never reconstructs candidates from payload equality.
pub(crate) fn run_regret<S, A, D, BestCb>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    bound_unassigned: &[SourceElement<A::Element>],
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, BestCb>,
) where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut phase_scope =
        PhaseScope::with_phase_type(solver_scope, 0, "Regret-Insertion Construction");
    let source_count = source_index.source_count();
    let entity_count = access.entity_count(phase_scope.score_director().working_solution());
    if entity_count == 0 || source_count == 0 {
        phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }
    if bound_unassigned.is_empty() {
        tracing::info!("All elements already assigned, skipping regret insertion");
        phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let mut unassigned = bound_unassigned.to_vec();
    let solution = phase_scope.score_director().working_solution();
    unassigned.sort_by_key(|entry| {
        (
            access.construction_order_key(solution, &entry.element),
            entry.source_index,
        )
    });
    let downstream_by_source =
        precedence_downstream_by_source(access, solution, source_index, &unassigned);

    if solve_oversized_owner_restricted(
        access,
        &mut phase_scope,
        source_index,
        &unassigned,
        entity_count,
        control_policy,
    ) {
        phase_scope.update_best_solution();
        return;
    }

    while !unassigned.is_empty() {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            break;
        }
        let mut best_choice: Option<(
            RegretValue<S::Score>,
            usize,
            usize,
            usize,
            S::Score,
            usize,
            Option<CandidateTracePullToken>,
        )> = None;
        let mut interrupted = false;
        for (list_index, entry) in unassigned.iter().enumerate() {
            if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                interrupted = true;
                break;
            }
            let result = match evaluate_regret(
                access,
                entry,
                entity_count,
                control_policy,
                &mut phase_scope,
            ) {
                RegretEvaluation::Complete(result) => result,
                RegretEvaluation::Interrupted => {
                    interrupted = true;
                    break;
                }
            };
            let Some((regret, entity_index, position, score, trace_token)) = result else {
                continue;
            };
            let downstream = downstream_by_source
                .as_ref()
                .and_then(|values| values.get(entry.source_index))
                .copied()
                .unwrap_or(0);
            let better = match best_choice {
                None => true,
                Some((best_regret, _, _, _, best_score, best_downstream, _)) => {
                    regret_choice_is_better_with_downstream(
                        regret,
                        score,
                        downstream,
                        best_regret,
                        best_score,
                        best_downstream,
                    )
                }
            };
            if better {
                if let Some((_, _, _, _, _, _, Some(token))) = best_choice.take() {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                best_choice = Some((
                    regret,
                    list_index,
                    entity_index,
                    position,
                    score,
                    downstream,
                    trace_token,
                ));
            } else if let Some(token) = trace_token {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
            }
        }
        if interrupted {
            if let Some((_, _, _, _, _, _, Some(token))) = best_choice.take() {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
            }
            break;
        }
        let Some((_, list_index, entity_index, position, score, _, trace_token)) = best_choice
        else {
            tracing::warn!("No valid insertion found for remaining elements, stopping");
            break;
        };
        let entry = unassigned.remove(list_index);
        let mut step_scope = StepScope::new_with_control_policy(&mut phase_scope, control_policy);
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
        }
        step_scope.apply_committed_change(|director| {
            apply_insertion(access, &entry, entity_index, position, director);
        });
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
        }
        step_scope.set_step_score(score);
        step_scope.complete();
    }
    phase_scope.update_best_solution();
}
