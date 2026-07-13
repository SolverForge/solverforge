//! Canonical Clarke-Wright completion-insertion pass.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::route_state::{route_elements, route_values, ConstructedRoute};
use super::{
    insertion_delta, owner_allows, route_owner_allows, ClarkeWrightAccess, CompletionAssignment,
    RuntimeListSourceIndex,
};
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTracePullToken,
    CandidateTraceSource,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn complete_routes_by_insertion<S, A, D, BestCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    owner_slots: &[super::owner_assignment::OwnerSlot],
    routes: &[ConstructedRoute],
    entity_count: usize,
    control_policy: StepControlPolicy,
) -> Option<Vec<(usize, Vec<usize>)>>
where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut assignments = owner_slots
        .iter()
        .map(|slot| CompletionAssignment {
            owner_idx: slot.owner_idx,
            route_indices: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut element_order = Vec::new();

    for (route_idx, route) in routes.iter().enumerate() {
        if route.visits.is_empty() {
            continue;
        }
        for (visit_position, &element_idx) in route.visits.iter().enumerate() {
            let solution = phase_scope.score_director().working_solution();
            let element = source_index.element(element_idx);
            let value = [access.route_value(element)];
            let feasible_owner_count = owner_slots
                .iter()
                .filter(|slot| {
                    owner_allows(access, solution, entity_count, slot.owner_idx, element)
                        && access.savings_feasible(solution, slot.owner_idx, &value)
                })
                .count();
            if feasible_owner_count == 0 {
                return None;
            }
            element_order.push((feasible_owner_count, route_idx, visit_position, element_idx));
        }
    }
    element_order.sort_unstable();

    for (_, _, _, element_idx) in element_order {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            return None;
        }
        let mut best: Option<(i64, usize, usize, Option<CandidateTracePullToken>)> = None;
        for (assignment_idx, assignment) in assignments.iter().enumerate() {
            let owner_idx = assignment.owner_idx;
            let element = source_index.element(element_idx);
            if !owner_allows(
                access,
                phase_scope.score_director().working_solution(),
                entity_count,
                owner_idx,
                element,
            ) {
                continue;
            }

            for insert_idx in 0..=assignment.route_indices.len() {
                if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                    if let Some((_, _, _, Some(token))) = best.take() {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                    return None;
                }
                let trace_token = phase_scope.record_candidate_operation(
                    CandidateTraceSource::ListClarkeWrightCompletionInsertion,
                    None,
                    insert_idx,
                    Some(CandidateTraceConstructionTarget {
                        descriptor_index: access.descriptor_index(),
                        entity_index: owner_idx,
                    }),
                    access.descriptor_index(),
                    "clarke_wright_completion_insertion_trial",
                    [element_idx, owner_idx, insert_idx],
                );
                let mut candidate = assignment.route_indices.clone();
                candidate.insert(insert_idx, element_idx);
                let candidate_values = route_values(access, source_index, &candidate);
                if !access.savings_feasible(
                    phase_scope.score_director().working_solution(),
                    owner_idx,
                    &candidate_values,
                ) {
                    if let Some(token) = trace_token {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Evaluated,
                        );
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                    continue;
                }

                let candidate_elements = route_elements::<S, A>(source_index, &candidate);
                if !route_owner_allows(
                    access,
                    phase_scope.score_director().working_solution(),
                    entity_count,
                    owner_idx,
                    &candidate_elements,
                ) {
                    if let Some(token) = trace_token {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::Evaluated,
                        );
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                    continue;
                }

                let delta = insertion_delta(
                    phase_scope.score_director().working_solution(),
                    owner_idx,
                    &assignment.route_indices,
                    insert_idx,
                    element_idx,
                    access,
                    source_index,
                );
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
                }
                let candidate_key = (delta, assignment.route_indices.len(), assignment_idx);
                let best_key = best.map(|(delta, assignment_idx, _, _)| {
                    (
                        delta,
                        assignments[assignment_idx].route_indices.len(),
                        assignment_idx,
                    )
                });
                if best_key.is_none_or(|best_key| candidate_key < best_key) {
                    if let Some((_, _, _, Some(token))) = best.take() {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                    best = Some((delta, assignment_idx, insert_idx, trace_token));
                } else if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
            }
        }

        let (_, assignment_idx, insert_idx, trace_token) = best?;
        if let Some(token) = trace_token {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
        }
        assignments[assignment_idx]
            .route_indices
            .insert(insert_idx, element_idx);
        if let Some(token) = trace_token {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
        }
    }

    Some(
        assignments
            .into_iter()
            .filter_map(|assignment| {
                (!assignment.route_indices.is_empty()).then(|| {
                    (
                        assignment.owner_idx,
                        route_values(access, source_index, &assignment.route_indices),
                    )
                })
            })
            .collect(),
    )
}
