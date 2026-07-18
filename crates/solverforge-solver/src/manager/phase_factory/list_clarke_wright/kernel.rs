//! Canonical Clarke-Wright savings and merge execution.

use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::completion::complete_routes_by_insertion;
use super::owner_assignment::{
    feasible_owners_for_scored_elements, match_route_owners, owner_slots,
    representative_owner_slots,
};
use super::route_state::{
    route_elements, route_values, routes_match_owners_after_merge, ConstructedRoute,
};
use super::savings::{sort_savings, SavingsEntry};
use super::{owner_allows, ClarkeWrightAccess, RuntimeListSourceIndex, SourceElement};
use crate::phase::construction::run_construction_phase;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepControlPolicy, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

/// Runs the one canonical Clarke-Wright algorithm against a pre-bound source.
///
/// The caller must bind declarations and assigned values before entering this
/// function. That keeps source errors outside phase execution for direct and
/// retained runtime callers.
pub(crate) fn run_clarke_wright<S, A, D, BestCb>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    bound_unassigned: &[SourceElement<A::Element>],
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
        let _score = phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let available_entity_slots = (0..n_entities)
        .filter(|&entity_idx| access.route_len(solution, entity_idx) == 0)
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
        let _score = phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    if available_entity_slots.is_empty() {
        tracing::warn!(
            unassigned_elements = unassigned.len(),
            "ListClarkeWright found no empty entity slots for remaining work; leaving preassigned routes untouched"
        );
        let _score = phase_scope.score_director_mut().calculate_score();
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
    let mut routes = unassigned
        .iter()
        .map(|entry| {
            let singleton = [access.route_value(&entry.element)];
            let feasible_for_all_owners = owner_slots.iter().all(|slot| {
                access.savings_feasible(solution, slot.owner_idx, &singleton)
                    && owner_allows(access, solution, n_entities, slot.owner_idx, &entry.element)
            });
            ConstructedRoute::singleton(entry.source_index, feasible_for_all_owners)
        })
        .collect::<Vec<_>>();

    let mut route_of = vec![None; n_elements];
    for (route_idx, entry) in unassigned.iter().enumerate() {
        route_of[entry.source_index] = Some(route_idx);
    }

    let mut savings = Vec::with_capacity(
        n.saturating_mul(n.saturating_sub(1))
            .saturating_div(2)
            .saturating_mul(representative_owner_slots.len()),
    );
    let mut construction_interrupted = false;
    'savings_generation: for owner_slot in &representative_owner_slots {
        let owner_idx = owner_slot.owner_idx;
        for a in 0..n {
            if (a & 0x1F) == 0
                && control_policy.should_terminate_construction(phase_scope.solver_scope_mut())
            {
                construction_interrupted = true;
                break 'savings_generation;
            }
            for b in (a + 1)..n {
                let left = &unassigned[a];
                let right = &unassigned[b];
                let trace_token = phase_scope.record_candidate_operation(
                    CandidateTraceSource::ListClarkeWrightSavings,
                    None,
                    savings.len(),
                    Some(CandidateTraceConstructionTarget {
                        descriptor_index: access.descriptor_index(),
                        entity_index: owner_idx,
                    }),
                    access.descriptor_index(),
                    "clarke_wright_savings_pair",
                    [
                        owner_idx as u64,
                        trace_metric_class_by_value[&owner_slot.metric_class],
                        left.source_index as u64,
                        right.source_index as u64,
                    ],
                );
                let solution = phase_scope.score_director().working_solution();
                let depot = access.savings_depot(solution, owner_idx);
                let left_value = access.route_value(&left.element);
                let right_value = access.route_value(&right.element);
                let saving = super::sum_two_minus_one(
                    access.savings_distance(solution, owner_idx, depot, left_value),
                    access.savings_distance(solution, owner_idx, depot, right_value),
                    access.savings_distance(solution, owner_idx, left_value, right_value),
                );
                savings.push(SavingsEntry {
                    saving,
                    metric_class: owner_slot.metric_class,
                    left_idx: left.source_index,
                    right_idx: right.source_index,
                });
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
                if (savings.len() & 0x3FF) == 0
                    && control_policy.should_terminate_construction(phase_scope.solver_scope_mut())
                {
                    construction_interrupted = true;
                    break 'savings_generation;
                }
            }
        }
    }
    sort_savings(&mut savings);

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

            let mut test_ri = routes[ri].visits.clone();
            if test_ri.first() == Some(&entry.left_idx) {
                test_ri.reverse();
            }
            let mut test_rj = routes[rj].visits.clone();
            if test_rj.last() == Some(&entry.right_idx) {
                test_rj.reverse();
            }
            let candidate_indices = test_ri.iter().chain(&test_rj).copied().collect::<Vec<_>>();
            let solution = phase_scope.score_director().working_solution();
            let candidate_route = route_values(access, source_index, &candidate_indices);
            let candidate_elements = route_elements::<S, A>(source_index, &candidate_indices);
            let candidate_feasible_owners = feasible_owners_for_scored_elements(
                access,
                solution,
                &owner_slots,
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
            let candidate_feasible_for_all_metric_class_owners = candidate_feasible_owners.len()
                == owner_slots
                    .iter()
                    .filter(|slot| slot.metric_class == entry.metric_class)
                    .count();
            if !routes_match_owners_after_merge(
                access,
                solution,
                source_index,
                &routes,
                ri,
                rj,
                &candidate_indices,
                entry.metric_class,
                candidate_feasible_for_all_metric_class_owners,
                &owner_slots,
                n_entities,
            ) {
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                continue;
            }

            if let Some(token) = trace_token {
                phase_scope
                    .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
            }
            routes[ri].visits = test_ri;
            routes[ri].scored_metric_class = Some(entry.metric_class);
            routes[ri].feasible_for_all_owners = false;
            routes[ri].feasible_for_all_metric_class_owners =
                candidate_feasible_for_all_metric_class_owners;
            routes[rj].visits.clear();
            routes[rj].scored_metric_class = None;
            routes[rj].feasible_for_all_owners = false;
            routes[rj].feasible_for_all_metric_class_owners = false;
            for &source_idx in &test_rj {
                route_of[source_idx] = Some(ri);
            }
            routes[ri].visits.extend(test_rj);
            if let Some(token) = trace_token {
                phase_scope
                    .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
            }
            merged_in_pass = true;
        }
        if construction_interrupted || !merged_in_pass {
            break;
        }
        merge_pass = merge_pass.saturating_add(1);
    }

    let non_empty = routes
        .into_iter()
        .filter(|route| !route.visits.is_empty())
        .collect::<Vec<_>>();
    if construction_interrupted {
        let _score = phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }
    let solution = phase_scope.score_director().working_solution();
    let constructed_route_count = non_empty.len();
    let mut assignable_routes = Vec::with_capacity(constructed_route_count);
    let mut feasible_sets = Vec::with_capacity(constructed_route_count);
    let mut owner_ineligible_routes = 0usize;
    for route in &non_empty {
        let values = route_values(access, source_index, &route.visits);
        let elements = route_elements::<S, A>(source_index, &route.visits);
        let feasible_owners = feasible_owners_for_scored_elements(
            access,
            solution,
            &owner_slots,
            &values,
            &elements,
            route.scored_metric_class,
            n_entities,
        );
        if feasible_owners.is_empty() {
            owner_ineligible_routes += 1;
            continue;
        }
        assignable_routes.push(route.clone());
        feasible_sets.push(feasible_owners);
    }
    let route_to_owner = match_route_owners(&feasible_sets);
    let matched_count = route_to_owner
        .iter()
        .filter(|owner| owner.is_some())
        .count();
    let completion_routes = if matched_count < assignable_routes.len()
        && !construction_interrupted
        && owner_ineligible_routes == 0
    {
        complete_routes_by_insertion(
            phase_scope,
            access,
            source_index,
            &owner_slots,
            &non_empty,
            n_entities,
            control_policy,
        )
    } else {
        None
    };

    if matched_count < assignable_routes.len()
        && !construction_interrupted
        && completion_routes.is_none()
    {
        tracing::warn!(
            constructed_routes = constructed_route_count,
            owner_ineligible_routes,
            available_slots = available_entity_slots.len(),
            matched_routes = matched_count,
            "ListClarkeWright could not match every constructed route to a feasible empty entity"
        );
        let _score = phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    if let Some(completed_routes) = completion_routes {
        let descriptor_index = access.descriptor_index();
        let mut step_scope = StepScope::new_with_control_policy(phase_scope, control_policy);
        step_scope.apply_committed_change(|director| {
            for (entity_idx, route) in completed_routes {
                director.before_variable_changed(descriptor_index, entity_idx);
                access.replace_route(director.working_solution_mut(), entity_idx, route);
                director.after_variable_changed(descriptor_index, entity_idx);
            }
        });
        let step_score = step_scope.calculate_score();
        step_scope.set_step_score(step_score);
        step_scope.complete();
    } else if matched_count > 0 {
        let descriptor_index = access.descriptor_index();
        let mut step_scope = StepScope::new_with_control_policy(phase_scope, control_policy);
        step_scope.apply_committed_change(|director| {
            for (index_route, entity_idx) in assignable_routes.into_iter().zip(route_to_owner) {
                let Some(entity_idx) = entity_idx else {
                    continue;
                };
                director.before_variable_changed(descriptor_index, entity_idx);
                let route = route_values(access, source_index, &index_route.visits);
                access.replace_route(director.working_solution_mut(), entity_idx, route);
                director.after_variable_changed(descriptor_index, entity_idx);
            }
        });
        let step_score = step_scope.calculate_score();
        step_scope.set_step_score(step_score);
        step_scope.complete();
    }
    phase_scope.update_best_solution();
}
