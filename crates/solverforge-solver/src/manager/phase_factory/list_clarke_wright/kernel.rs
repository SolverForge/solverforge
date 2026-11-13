//! Canonical Clarke-Wright savings and merge execution.

use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::commit::{commit_complete_routes, try_commit_balanced_assignment};
use super::completion::{complete_routes_by_insertion, CompletionSelection};
use super::owner_admission::{MergeProposal, OwnerAdmission, OwnerQuery};
use super::owner_assignment::{
    assign_buffered_routes, feasible_owners_for_scored_elements, match_route_owners, owner_slots,
    representative_owner_slots, SavingsFeasibility,
};
use super::route_state::{route_elements_into, route_values, route_values_into, ConstructedRoute};
use super::savings::{generate_savings, sort_savings};
use super::{element_restriction, ClarkeWrightAccess, RuntimeListSourceIndex, SourceElement};
use crate::phase::construction::{
    record_construction_candidate, run_construction_phase, PendingConstructionMoveTelemetry,
};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepControlPolicy};
use crate::stats::{CandidateTraceDisposition, CandidateTraceSource};

/// Runs the one canonical Clarke-Wright algorithm against a pre-bound source.
///
/// The caller must bind declarations and assigned values before entering this
/// function. That keeps source errors outside phase execution for direct and
/// retained runtime callers.
pub(crate) fn run_clarke_wright<S, A, D, BestCb>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    bound_unassigned: &[SourceElement<A::Element>],
    savings_neighbors: Option<usize>,
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, BestCb>,
) where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    run_construction_phase(
        solver_scope,
        0,
        "Clarke-Wright Construction",
        |phase_scope| {
            run_clarke_wright_in_phase(
                access,
                source_index,
                bound_unassigned,
                savings_neighbors,
                control_policy,
                phase_scope,
            );
        },
    );
}

fn run_clarke_wright_in_phase<S, A, D, BestCb>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    bound_unassigned: &[SourceElement<A::Element>],
    savings_neighbors: Option<usize>,
    control_policy: StepControlPolicy,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let solution = phase_scope.score_director().working_solution();
    let n_entities = access.entity_count(solution);
    let n_elements = source_index.source_count();
    if n_entities == 0 || n_elements == 0 {
        phase_scope.calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let available_entity_slots = (0..n_entities)
        .filter(|&entity_idx| {
            access.route_len(solution, entity_idx) == 0
                && !crate::pinning::entity_is_pinned(
                    phase_scope.score_director(),
                    access.descriptor_index(),
                    entity_idx,
                )
        })
        .collect::<Vec<_>>();
    let depot_values = available_entity_slots
        .iter()
        .map(|&entity_idx| access.savings_depot(solution, entity_idx))
        .collect::<HashSet<_>>();
    let unassigned = bound_unassigned
        .iter()
        .filter(|entry| !depot_values.contains(&access.route_value(&entry.element)))
        .cloned()
        .collect::<Vec<_>>();
    if unassigned.is_empty() {
        tracing::info!("All elements already assigned, skipping Clarke-Wright construction");
        phase_scope.calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    if available_entity_slots.is_empty() {
        tracing::warn!(
            unassigned_elements = unassigned.len(),
            "ListClarkeWright found no empty entity slots for remaining work; leaving preassigned routes untouched"
        );
        phase_scope.calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let owner_slots = owner_slots(access, solution, &available_entity_slots);
    let representative_owner_slots = representative_owner_slots(&owner_slots);
    let mut trace_metric_class_by_value = HashMap::new();
    for owner_slot in &owner_slots {
        let next_id = trace_metric_class_by_value.len() as u64;
        trace_metric_class_by_value
            .entry(owner_slot.metric_class)
            .or_insert(next_id);
    }
    let n = unassigned.len();
    let mut feasibility = SavingsFeasibility::new(access, &owner_slots);
    let mut routes = Vec::with_capacity(n);
    for entry in &unassigned {
        let singleton = [access.route_value(&entry.element)];
        feasibility.begin_route();
        let restriction = element_restriction(access, solution, n_entities, &entry.element);
        let feasible_for_all_owners = (0..owner_slots.len()).all(|slot_position| {
            feasibility.feasible(access, solution, &owner_slots, slot_position, &singleton)
                && restriction.allows(owner_slots[slot_position].owner_idx)
        });
        routes.push(ConstructedRoute::singleton(
            entry.source_index,
            feasible_for_all_owners,
        ));
    }

    let mut route_of = vec![None; n_elements];
    for (route_idx, entry) in unassigned.iter().enumerate() {
        route_of[entry.source_index] = Some(route_idx);
    }

    try_commit_balanced_assignment(
        phase_scope,
        access,
        source_index,
        &owner_slots,
        &mut feasibility,
        &routes,
        n_entities,
        &available_entity_slots,
        control_policy,
    );
    if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
        phase_scope.calculate_score();
        return;
    }

    let (mut savings, mut construction_interrupted) = generate_savings(
        phase_scope,
        access,
        &unassigned,
        &representative_owner_slots,
        &trace_metric_class_by_value,
        savings_neighbors,
        control_policy,
    );
    sort_savings(&mut savings);
    let mut pending_move_telemetry = PendingConstructionMoveTelemetry::default();
    let mut admission = OwnerAdmission::new(&routes, &owner_slots);
    let mut candidate_indices = Vec::new();
    let mut candidate_route = Vec::new();
    let mut candidate_elements = Vec::new();

    let mut merge_pass = 0usize;
    loop {
        let mut merged_in_pass = false;
        for (merge_idx, entry) in savings.iter().enumerate() {
            if (merge_idx & 0xFF) == 0
                && control_policy.should_terminate_construction(phase_scope.solver_scope_mut())
            {
                construction_interrupted = true;
                break;
            }
            let trace_token = phase_scope.record_candidate_operation(
                CandidateTraceSource::ListClarkeWrightMerge,
                None,
                merge_idx,
                None,
                access.descriptor_index(),
                "clarke_wright_merge_trial",
                [
                    merge_pass as u64,
                    trace_metric_class_by_value[&entry.metric_class],
                    entry.left_idx as u64,
                    entry.right_idx as u64,
                    entry.saving as u64,
                ],
            );
            if let Some(token) = trace_token {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::Evaluated,
                );
            }
            record_construction_candidate(
                phase_scope,
                std::time::Duration::ZERO,
                std::time::Duration::ZERO,
            );
            let (Some(ri), Some(rj)) = (route_of[entry.left_idx], route_of[entry.right_idx]) else {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                continue;
            };
            if ri == rj
                || !routes[ri].can_merge_for_metric_class(entry.metric_class)
                || !routes[rj].can_merge_for_metric_class(entry.metric_class)
            {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                continue;
            }
            let i_is_endpoint = routes[ri].visits.first() == Some(&entry.left_idx)
                || routes[ri].visits.last() == Some(&entry.left_idx);
            let j_is_endpoint = routes[rj].visits.first() == Some(&entry.right_idx)
                || routes[rj].visits.last() == Some(&entry.right_idx);
            if !i_is_endpoint || !j_is_endpoint {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                continue;
            }

            // The merged route is route ri ending at the left element, then
            // route rj starting at the right element.
            candidate_indices.clear();
            let ri_visits = &routes[ri].visits;
            if ri_visits.first() == Some(&entry.left_idx) {
                candidate_indices.extend(ri_visits.iter().rev());
            } else {
                candidate_indices.extend_from_slice(ri_visits);
            }
            let merged_ri_len = candidate_indices.len();
            let rj_visits = &routes[rj].visits;
            if rj_visits.last() == Some(&entry.right_idx) {
                candidate_indices.extend(rj_visits.iter().rev());
            } else {
                candidate_indices.extend_from_slice(rj_visits);
            }
            let solution = phase_scope.score_director().working_solution();
            route_values_into(
                access,
                source_index,
                &candidate_indices,
                &mut candidate_route,
            );
            route_elements_into::<S, A>(source_index, &candidate_indices, &mut candidate_elements);
            let candidate_feasible_owners = feasible_owners_for_scored_elements(
                access,
                solution,
                &owner_slots,
                &mut feasibility,
                &candidate_route,
                &candidate_elements,
                Some(entry.metric_class),
                n_entities,
            );
            if candidate_feasible_owners.is_empty() {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                continue;
            }
            let proposal = MergeProposal {
                merged: ri,
                removed: rj,
                metric_class: entry.metric_class,
                feasible_for_all_metric_class_owners: candidate_feasible_owners.len()
                    == admission.class_owner_count(entry.metric_class),
                feasible_owners: &candidate_feasible_owners,
            };
            let query = OwnerQuery {
                access,
                solution,
                source_index,
                owner_slots: &owner_slots,
                entity_count: n_entities,
            };
            if !admission.admits(&query, &routes, &mut feasibility, &proposal) {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                continue;
            }

            admission.commit(&routes, &proposal);
            let feasible_for_all_metric_class_owners =
                proposal.feasible_for_all_metric_class_owners;
            pending_move_telemetry.record_accepted(phase_scope, trace_token);
            routes[ri].visits.clear();
            routes[ri].visits.extend_from_slice(&candidate_indices);
            routes[ri].scored_metric_class = Some(entry.metric_class);
            routes[ri].feasible_for_all_owners = false;
            routes[ri].feasible_for_all_metric_class_owners = feasible_for_all_metric_class_owners;
            routes[rj].visits.clear();
            routes[rj].scored_metric_class = None;
            routes[rj].feasible_for_all_owners = false;
            routes[rj].feasible_for_all_metric_class_owners = false;
            for &source_idx in &candidate_indices[merged_ri_len..] {
                route_of[source_idx] = Some(ri);
            }
            merged_in_pass = true;
        }
        if construction_interrupted || !merged_in_pass {
            break;
        }
        merge_pass = merge_pass.saturating_add(1);
    }

    if construction_interrupted {
        let non_empty = routes
            .into_iter()
            .filter(|route| !route.visits.is_empty())
            .collect::<Vec<_>>();
        // A merge leaves one buffered route empty, so a survivor count below the
        // element count is exactly "the loop merged something". Every merge it
        // admitted left the whole buffered route set matchable by metric class,
        // so those merges are a strictly better complete assignment than the
        // seed. Publish them instead of letting a tight budget throw the
        // construction work away.
        if non_empty.len() < n {
            if let Some(completed_routes) = assign_buffered_routes(
                access,
                source_index,
                &owner_slots,
                &available_entity_slots,
                &non_empty,
            ) {
                commit_complete_routes(
                    phase_scope,
                    access,
                    &available_entity_slots,
                    completed_routes,
                    control_policy,
                    pending_move_telemetry,
                );
                return;
            }
        }
        pending_move_telemetry.record_discarded(phase_scope);
        phase_scope.calculate_score();
        return;
    }
    let solution = phase_scope.score_director().working_solution();
    let query = OwnerQuery {
        access,
        solution,
        source_index,
        owner_slots: &owner_slots,
        entity_count: n_entities,
    };
    let route_owner_sets = admission.into_owner_sets(&query, &routes, &mut feasibility);
    let non_empty = routes
        .into_iter()
        .filter(|route| !route.visits.is_empty())
        .collect::<Vec<_>>();
    let constructed_route_count = non_empty.len();
    let mut assignable_routes = Vec::with_capacity(constructed_route_count);
    let mut feasible_sets = Vec::with_capacity(constructed_route_count);
    let mut owner_ineligible_routes = 0usize;
    for (route_index, feasible_owners) in route_owner_sets.into_iter().enumerate() {
        if feasible_owners.is_empty() {
            owner_ineligible_routes += 1;
            continue;
        }
        assignable_routes.push(route_index);
        feasible_sets.push(feasible_owners);
    }
    let route_to_owner = match_route_owners(&feasible_sets);
    let matched_count = route_to_owner
        .iter()
        .filter(|owner| owner.is_some())
        .count();
    let completion_routes =
        if matched_count < assignable_routes.len() && owner_ineligible_routes == 0 {
            complete_routes_by_insertion(
                phase_scope,
                access,
                source_index,
                &owner_slots,
                &mut feasibility,
                &non_empty,
                n_entities,
                CompletionSelection::Cheapest,
                control_policy,
                &mut pending_move_telemetry,
            )
        } else {
            None
        };

    if matched_count < assignable_routes.len() && completion_routes.is_none() {
        tracing::warn!(
            constructed_routes = constructed_route_count,
            owner_ineligible_routes,
            available_slots = available_entity_slots.len(),
            matched_routes = matched_count,
            "ListClarkeWright could not match every constructed route to a feasible empty entity"
        );
        pending_move_telemetry.record_discarded(phase_scope);
        phase_scope.calculate_score();
        return;
    }

    if let Some(completed_routes) = completion_routes {
        commit_complete_routes(
            phase_scope,
            access,
            &available_entity_slots,
            completed_routes,
            control_policy,
            pending_move_telemetry,
        );
    } else if matched_count > 0 {
        let completed_routes = assignable_routes
            .into_iter()
            .zip(route_to_owner)
            .filter_map(|(route_index, entity_idx)| {
                entity_idx.map(|entity_idx| {
                    let route = route_values(access, source_index, &non_empty[route_index].visits);
                    (entity_idx, route)
                })
            })
            .collect();
        commit_complete_routes(
            phase_scope,
            access,
            &available_entity_slots,
            completed_routes,
            control_policy,
            pending_move_telemetry,
        );
    } else {
        pending_move_telemetry.record_discarded(phase_scope);
    }
    phase_scope.update_best_solution();
}
