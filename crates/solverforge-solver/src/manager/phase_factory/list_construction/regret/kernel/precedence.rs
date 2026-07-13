//! Precedence-aware dispatch and bounded owner-restricted regret work.

use std::cmp::Reverse;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::evaluation::{apply_insertion, evaluate_owner_regret};
use super::{
    precedence_frontier_choice_is_better, source_position_by_index, RegretAccess, RegretEvaluation,
    RegretValue, OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET,
};
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::list_placement::OwnerRestriction;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy, StepScope};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken};

struct PrecedenceGraph {
    successors: Vec<Vec<usize>>,
    predecessor_counts: Vec<usize>,
    durations: Vec<usize>,
}

fn build_precedence_graph<S, A>(
    access: &A,
    solution: &S,
    source_index: &RuntimeListSourceIndex<A::Element>,
    entries: &[SourceElement<A::Element>],
) -> Option<PrecedenceGraph>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
{
    if entries.is_empty() {
        return Some(PrecedenceGraph {
            successors: Vec::new(),
            predecessor_counts: Vec::new(),
            durations: Vec::new(),
        });
    }
    let positions = source_position_by_index(source_index.source_count(), entries)?;
    let durations = entries
        .iter()
        .map(|entry| access.precedence_duration(solution, &entry.element))
        .collect::<Option<Vec<_>>>()?;
    let mut successors = vec![Vec::new(); entries.len()];
    let mut predecessor_counts = vec![0usize; entries.len()];
    let mut successor_sources = Vec::new();
    for (from_position, entry) in entries.iter().enumerate() {
        successor_sources.clear();
        if !access.extend_precedence_successor_source_indices(
            solution,
            &entry.element,
            source_index,
            &mut successor_sources,
        ) {
            return None;
        }
        for successor_source in &successor_sources {
            if let Some(Some(to_position)) = positions.get(*successor_source) {
                successors[from_position].push(*to_position);
                predecessor_counts[*to_position] += 1;
            }
        }
    }
    Some(PrecedenceGraph {
        successors,
        predecessor_counts,
        durations,
    })
}

pub(super) fn topological_order(
    successors: &[Vec<usize>],
    predecessor_counts: &[usize],
) -> Option<Vec<usize>> {
    let mut remaining = predecessor_counts.to_vec();
    let mut ready = remaining
        .iter()
        .enumerate()
        .filter_map(|(index, &count)| (count == 0).then_some(index))
        .collect::<Vec<_>>();
    let mut order = Vec::with_capacity(successors.len());
    while let Some(index) = ready.pop() {
        order.push(index);
        for &successor in &successors[index] {
            remaining[successor] -= 1;
            if remaining[successor] == 0 {
                ready.push(successor);
            }
        }
    }
    (order.len() == successors.len()).then_some(order)
}

pub(super) fn downstream_durations(
    successors: &[Vec<usize>],
    durations: &[usize],
    topological_order: &[usize],
) -> Vec<usize> {
    let mut downstream = durations.to_vec();
    for &index in topological_order.iter().rev() {
        let tail = successors[index]
            .iter()
            .map(|&successor| downstream[successor])
            .max()
            .unwrap_or(0);
        downstream[index] = durations[index].saturating_add(tail);
    }
    downstream
}

pub(super) fn precedence_downstream_by_source<S, A>(
    access: &A,
    solution: &S,
    source_index: &RuntimeListSourceIndex<A::Element>,
    entries: &[SourceElement<A::Element>],
) -> Option<Vec<usize>>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
{
    let graph = build_precedence_graph(access, solution, source_index, entries)?;
    let order = topological_order(&graph.successors, &graph.predecessor_counts)?;
    let downstream = downstream_durations(&graph.successors, &graph.durations, &order);
    let mut by_source = vec![0usize; source_index.source_count()];
    for (position, entry) in entries.iter().enumerate() {
        *by_source.get_mut(entry.source_index)? = downstream[position];
    }
    Some(by_source)
}

fn fixed_owners<S, A>(
    access: &A,
    solution: &S,
    entries: &[SourceElement<A::Element>],
    entity_count: usize,
) -> Option<Vec<usize>>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
{
    entries
        .iter()
        .map(
            |entry| match access.owner_restriction(solution, entity_count, &entry.element) {
                OwnerRestriction::Fixed(owner) if owner < entity_count => Some(owner),
                OwnerRestriction::Unrestricted
                | OwnerRestriction::Fixed(_)
                | OwnerRestriction::Invalid => None,
            },
        )
        .collect()
}

pub(super) fn precedence_frontier_regret_trial_count(
    successors: &[Vec<usize>],
    predecessor_counts: &[usize],
    owners: &[usize],
    owner_count: usize,
) -> Option<usize> {
    if successors.len() != predecessor_counts.len() || successors.len() != owners.len() {
        return None;
    }
    let mut remaining = predecessor_counts.to_vec();
    let mut ready = remaining
        .iter()
        .enumerate()
        .filter_map(|(index, &count)| (count == 0).then_some(index))
        .collect::<Vec<_>>();
    let mut owner_lengths = vec![0usize; owner_count];
    let mut processed = 0usize;
    let mut trials = 0usize;
    while let Some(index) = ready.pop() {
        for &ready_index in &ready {
            let owner = *owners.get(ready_index)?;
            if owner >= owner_count {
                return None;
            }
            trials = trials.saturating_add(owner_lengths[owner].saturating_add(1));
        }
        let owner = *owners.get(index)?;
        if owner >= owner_count {
            return None;
        }
        trials = trials.saturating_add(owner_lengths[owner].saturating_add(1));
        owner_lengths[owner] = owner_lengths[owner].saturating_add(1);
        processed += 1;
        for &successor in &successors[index] {
            remaining[successor] -= 1;
            if remaining[successor] == 0 {
                ready.push(successor);
            }
        }
    }
    (processed == successors.len()).then_some(trials)
}

pub(super) fn solve_precedence_frontier_regret<S, A, D, BestCb>(
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
    if entries.is_empty() {
        return true;
    }
    let solution = phase_scope.score_director().working_solution();
    let Some(owners) = fixed_owners(access, solution, entries, entity_count) else {
        return false;
    };
    let Some(graph) = build_precedence_graph(access, solution, source_index, entries) else {
        return false;
    };
    let Some(trials) = precedence_frontier_regret_trial_count(
        &graph.successors,
        &graph.predecessor_counts,
        &owners,
        entity_count,
    ) else {
        return false;
    };
    if trials.saturating_mul(entries.len()) > OWNER_RESTRICTED_BEST_INSERTION_WORK_BUDGET {
        return false;
    }
    let Some(topological) = topological_order(&graph.successors, &graph.predecessor_counts) else {
        return false;
    };
    let downstream = downstream_durations(&graph.successors, &graph.durations, &topological);
    let mut remaining = graph.predecessor_counts;
    let mut ready = topological
        .iter()
        .copied()
        .filter(|&index| remaining[index] == 0)
        .collect::<Vec<_>>();
    let mut inserted = 0usize;

    while !ready.is_empty() {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            break;
        }
        let mut best_choice: Option<(
            RegretValue<S::Score>,
            S::Score,
            usize,
            usize,
            usize,
            usize,
            Option<CandidateTracePullToken>,
        )> = None;
        let mut interrupted = false;
        for (ready_position, &entry_position) in ready.iter().enumerate() {
            if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                interrupted = true;
                break;
            }
            let entry = &entries[entry_position];
            let owner = owners[entry_position];
            let result =
                match evaluate_owner_regret(access, entry, owner, control_policy, phase_scope) {
                    RegretEvaluation::Complete(result) => result,
                    RegretEvaluation::Interrupted => {
                        interrupted = true;
                        break;
                    }
                };
            let Some((regret, position, score, trace_token)) = result else {
                continue;
            };
            let better = match best_choice {
                None => true,
                Some((best_regret, best_score, best_downstream, _, _, _, _)) => {
                    precedence_frontier_choice_is_better(
                        downstream[entry_position],
                        regret,
                        score,
                        best_downstream,
                        best_regret,
                        best_score,
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
                    score,
                    downstream[entry_position],
                    ready_position,
                    entry_position,
                    position,
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
        let Some((_, score, _, ready_position, entry_position, position, trace_token)) =
            best_choice
        else {
            return false;
        };
        ready.remove(ready_position);
        let entry = &entries[entry_position];
        let owner = owners[entry_position];
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
        inserted += 1;
        for &successor in &graph.successors[entry_position] {
            remaining[successor] -= 1;
            if remaining[successor] == 0 {
                ready.push(successor);
            }
        }
    }
    inserted == entries.len()
}

pub(super) fn precedence_dispatch_order<S, A>(
    access: &A,
    solution: &S,
    source_index: &RuntimeListSourceIndex<A::Element>,
    entries: &[SourceElement<A::Element>],
    entity_count: usize,
) -> Option<Vec<(usize, usize)>>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
{
    if entries.is_empty() {
        return Some(Vec::new());
    }
    let owners = fixed_owners(access, solution, entries, entity_count)?;
    let graph = build_precedence_graph(access, solution, source_index, entries)?;
    let topological = topological_order(&graph.successors, &graph.predecessor_counts)?;
    let downstream = downstream_durations(&graph.successors, &graph.durations, &topological);
    let mut remaining = graph.predecessor_counts;
    let mut predecessor_ready = vec![0usize; entries.len()];
    let mut owner_ready = vec![0usize; entity_count];
    let mut ready = topological
        .iter()
        .copied()
        .filter(|&index| remaining[index] == 0)
        .collect::<Vec<_>>();
    let mut order = Vec::with_capacity(entries.len());
    while !ready.is_empty() {
        let ready_position = ready
            .iter()
            .enumerate()
            .min_by_key(|&(_, &entry_position)| {
                let owner = owners[entry_position];
                let start = predecessor_ready[entry_position].max(owner_ready[owner]);
                let finish = start.saturating_add(graph.durations[entry_position]);
                let order_key =
                    access.construction_order_key(solution, &entries[entry_position].element);
                (
                    start,
                    Reverse(downstream[entry_position]),
                    order_key,
                    finish,
                    entry_position,
                )
            })
            .map(|(position, _)| position)?;
        let entry_position = ready.remove(ready_position);
        let owner = owners[entry_position];
        let start = predecessor_ready[entry_position].max(owner_ready[owner]);
        let finish = start.saturating_add(graph.durations[entry_position]);
        owner_ready[owner] = finish;
        order.push((entry_position, owner));
        for &successor in &graph.successors[entry_position] {
            predecessor_ready[successor] = predecessor_ready[successor].max(finish);
            remaining[successor] -= 1;
            if remaining[successor] == 0 {
                ready.push(successor);
            }
        }
    }
    (order.len() == entries.len()).then_some(order)
}
