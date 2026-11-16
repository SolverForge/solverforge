//! Cheapest insertion of a list with element groups.
//!
//! Units go in construction order; each unit takes its best bounded joint
//! placement over every owner it allows, so a group's members are always
//! placed together.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::super::groups::{
    commit_group, construction_units, evaluate_group_trial, keep_best_token, unit_owners,
};
use super::super::ScoredListConstructionAccess;
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::list_groups::{
    search_joint_placement, JointBounds, JointSearch, ListElementGroups, ListGroupPlacement,
};
use crate::phase::construction::run_construction_phase;
use crate::scope::{ProgressCallback, SolverScope, StepControlPolicy};
use crate::stats::{CandidateTraceDisposition, CandidateTraceSource};

/// Runs grouped cheapest insertion as one construction phase.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_cheapest_grouped<S, A, D, BestCb>(
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    bound_unassigned: &[SourceElement<A::Element>],
    groups: &ListElementGroups,
    placement: &ListGroupPlacement,
    bounds: JointBounds,
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, BestCb>,
) where
    S: PlanningSolution,
    S::Score: Copy + Ord,
    A: ScoredListConstructionAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    run_construction_phase(
        solver_scope,
        0,
        "Cheapest-Insertion Construction",
        |phase_scope| {
            let solution = phase_scope.score_director().working_solution();
            let mut ordered = bound_unassigned.to_vec();
            ordered.sort_by_key(|entry| {
                (
                    access.construction_order_key(solution, &entry.element),
                    entry.source_index,
                )
            });
            let units =
                construction_units(access, solution, source_index, &ordered, groups, placement);
            if units.is_empty() || access.entity_count(solution) == 0 {
                phase_scope.calculate_score();
                phase_scope.update_best_solution();
                return;
            }
            for unit in &units {
                if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                    break;
                }
                let (owners, lens) = unit_owners(access, unit.restriction, phase_scope);
                let first_source = unit.members[0].source_index;
                let mut members = unit.elements();
                let mut best_token = None;
                let mut trial = 0;
                let search = search_joint_placement(
                    &owners,
                    members.len(),
                    |entity| lens[entity],
                    bounds,
                    |entity, positions| {
                        if control_policy
                            .should_terminate_construction(phase_scope.solver_scope_mut())
                        {
                            return None;
                        }
                        let (score, token) = evaluate_group_trial(
                            access,
                            &mut members,
                            first_source,
                            entity,
                            positions,
                            CandidateTraceSource::ListCheapestInsertionTrial,
                            trial,
                            phase_scope,
                        );
                        trial += 1;
                        keep_best_token(&mut best_token, score, token, phase_scope);
                        Some(score)
                    },
                );
                let token = best_token.and_then(|(_, token)| token);
                match search {
                    JointSearch::Done {
                        best: Some(choice), ..
                    } => commit_group(
                        access,
                        members,
                        choice.entity,
                        &choice.positions,
                        choice.score,
                        token,
                        control_policy,
                        phase_scope,
                    ),
                    JointSearch::Done { best: None, .. } => {}
                    JointSearch::Interrupted => {
                        if let Some(token) = token {
                            phase_scope.record_candidate_trace_disposition(
                                token,
                                CandidateTraceDisposition::ForagerIgnored,
                            );
                        }
                        break;
                    }
                }
            }
            phase_scope.update_best_solution();
            phase_scope.promote_current_solution_on_score_tie();
        },
    );
}
