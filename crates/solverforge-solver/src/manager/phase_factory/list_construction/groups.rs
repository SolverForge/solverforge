//! Construction units of a list with element groups, and their joint trials.
//!
//! A unit is a whole group (its unassigned members, in declared order) or a
//! single ungrouped element. A unit's owner restriction combines every
//! member's restriction; members already assigned pin the unit to their
//! owner, so a partly assigned group completes on that owner.

use std::time::Instant;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::ScoredListConstructionAccess;
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::list_groups::{ListElementGroups, ListGroupPlacement};
use crate::list_placement::OwnerRestriction;
use crate::phase::construction::record_construction_candidate;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTracePullToken,
    CandidateTraceSource,
};

/// Elements placed together by grouped construction.
pub(crate) struct ConstructionUnit<E> {
    /// Unassigned members in declared order.
    pub(crate) members: SmallVec<[SourceElement<E>; 4]>,
    pub(crate) restriction: OwnerRestriction,
}

impl<E: Clone> ConstructionUnit<E> {
    pub(crate) fn elements(&self) -> SmallVec<[E; 4]> {
        self.members
            .iter()
            .map(|member| member.element.clone())
            .collect()
    }
}

/// Splits `ordered` unassigned elements into units, each unit at the place
/// of its first member in `ordered`.
pub(crate) fn construction_units<S, A>(
    access: &A,
    solution: &S,
    source_index: &RuntimeListSourceIndex<A::Element>,
    ordered: &[SourceElement<A::Element>],
    groups: &ListElementGroups,
    placement: &ListGroupPlacement,
) -> Vec<ConstructionUnit<A::Element>>
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
{
    let entity_count = access.entity_count(solution);
    let source_count = source_index.source_count();
    let mut entry_by_source = vec![None; source_count];
    for (entry, element) in ordered.iter().enumerate() {
        entry_by_source[element.source_index] = Some(entry);
    }
    let mut group_by_source = vec![None; source_count];
    for group in 0..groups.group_count() {
        for &key in groups.members(group) {
            if let Some(source) = source_index.source_index_for_key(key) {
                group_by_source[source] = Some(group);
            }
        }
    }
    let mut emitted = vec![false; groups.group_count()];
    let mut units = Vec::new();
    for element in ordered {
        let Some(group) = group_by_source[element.source_index] else {
            units.push(ConstructionUnit {
                members: SmallVec::from_elem(element.clone(), 1),
                restriction: access.owner_restriction(solution, entity_count, &element.element),
            });
            continue;
        };
        if std::mem::replace(&mut emitted[group], true) {
            continue;
        }
        let mut members = SmallVec::new();
        let mut restriction = OwnerRestriction::Unrestricted;
        for (member, &key) in groups.members(group).iter().enumerate() {
            let entry = source_index
                .source_index_for_key(key)
                .and_then(|source| entry_by_source[source]);
            match entry {
                Some(entry) => {
                    let member = &ordered[entry];
                    restriction = restriction.and(access.owner_restriction(
                        solution,
                        entity_count,
                        &member.element,
                    ));
                    members.push(member.clone());
                }
                None => {
                    if let Some((owner, _)) = placement.member_position(groups, group, member) {
                        restriction = restriction.and(OwnerRestriction::Fixed(owner));
                    }
                }
            }
        }
        units.push(ConstructionUnit {
            members,
            restriction,
        });
    }
    units
}

/// The unpinned owners a unit may go to, and every owner's list length.
pub(crate) fn unit_owners<S, A, D, BestCb>(
    access: &A,
    unit_restriction: OwnerRestriction,
    phase_scope: &PhaseScope<'_, '_, S, D, BestCb>,
) -> (SmallVec<[usize; 16]>, SmallVec<[usize; 16]>)
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let solution = phase_scope.score_director().working_solution();
    let entity_count = access.entity_count(solution);
    let owners = unit_restriction
        .candidates(entity_count)
        .filter(|&entity| {
            !crate::pinning::entity_is_pinned(
                phase_scope.score_director(),
                access.descriptor_index(),
                entity,
            )
        })
        .collect();
    let lens = (0..entity_count)
        .map(|entity| access.list_len(solution, entity))
        .collect();
    (owners, lens)
}

/// Scores one joint placement of `members` and takes them back.
#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_group_trial<S, A, D, BestCb>(
    access: &A,
    members: &mut SmallVec<[A::Element; 4]>,
    first_source: usize,
    entity: usize,
    positions: &[usize],
    source: CandidateTraceSource,
    candidate_index: usize,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) -> (S::Score, Option<CandidateTracePullToken>)
where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let descriptor_index = access.descriptor_index();
    let generation_started = Instant::now();
    let token = phase_scope.record_candidate_operation(
        source,
        None,
        candidate_index,
        Some(CandidateTraceConstructionTarget {
            descriptor_index,
            entity_index: entity,
        }),
        descriptor_index,
        "list_group_insertion_trial",
        [first_source as u64, entity as u64, positions[0] as u64],
    );
    let generation_duration = generation_started.elapsed();
    let evaluation_started = Instant::now();
    let score_state = phase_scope.score_director().snapshot_score_state();
    let director = phase_scope.score_director_mut();
    director.before_variable_changed(descriptor_index, entity);
    for (member, &position) in std::mem::take(members).into_iter().zip(positions) {
        access.insert_element(director.working_solution_mut(), entity, position, member);
    }
    director.after_variable_changed(descriptor_index, entity);
    let score = director.calculate_score();
    director.before_variable_changed(descriptor_index, entity);
    for &position in positions.iter().rev() {
        members.push(
            access
                .remove_element(director.working_solution_mut(), entity, position)
                .expect("a group trial removes the members it inserted"),
        );
    }
    members.reverse();
    director.after_variable_changed(descriptor_index, entity);
    phase_scope
        .score_director_mut()
        .restore_score_state(score_state);
    phase_scope.record_score_calculation();
    if let Some(token) = token {
        phase_scope.record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
    }
    record_construction_candidate(
        phase_scope,
        generation_duration,
        evaluation_started.elapsed(),
    );
    (score, token)
}

/// Keeps the trace token of the best trial so far and marks every other
/// trial ignored; ties keep the earlier trial.
pub(crate) fn keep_best_token<S, D, BestCb, Sc: Copy + Ord>(
    best: &mut Option<(Sc, Option<CandidateTracePullToken>)>,
    score: Sc,
    token: Option<CandidateTracePullToken>,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let ignored = match best {
        Some((best_score, _)) if score <= *best_score => token,
        _ => best.replace((score, token)).and_then(|(_, token)| token),
    };
    if let Some(token) = ignored {
        phase_scope
            .record_candidate_trace_disposition(token, CandidateTraceDisposition::ForagerIgnored);
    }
}

/// Commits a unit's members at `positions` of `entity` as one step.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_group<S, A, D, BestCb>(
    access: &A,
    members: SmallVec<[A::Element; 4]>,
    entity: usize,
    positions: &[usize],
    score: S::Score,
    token: Option<CandidateTracePullToken>,
    control_policy: StepControlPolicy,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) where
    S: PlanningSolution,
    A: ScoredListConstructionAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let descriptor_index = access.descriptor_index();
    let mut step_scope = StepScope::new_with_control_policy(phase_scope, control_policy);
    step_scope.phase_scope_mut().record_move_accepted();
    if let Some(token) = token {
        step_scope
            .phase_scope_mut()
            .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
    }
    step_scope.apply_committed_change(|director| {
        director.before_variable_changed(descriptor_index, entity);
        for (member, &position) in members.into_iter().zip(positions) {
            access.insert_element(director.working_solution_mut(), entity, position, member);
        }
        director.after_variable_changed(descriptor_index, entity);
    });
    step_scope.phase_scope_mut().record_move_applied();
    if let Some(token) = token {
        step_scope
            .phase_scope_mut()
            .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
    }
    step_scope.set_step_score(score);
    step_scope.complete();
}
