//! Regret insertion of a list with element groups.
//!
//! Each round scores every remaining unit's bounded joint placements and
//! commits the unit whose best placement beats its runner-up by the most, so
//! a group's members are always placed together.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::super::super::groups::{
    commit_group, construction_units, evaluate_group_trial, keep_best_token, unit_owners,
    ConstructionUnit,
};
use super::choice::regret_choice_is_better;
use super::{RegretAccess, RegretValue};
use crate::builder::context::{RuntimeListSourceIndex, SourceElement};
use crate::list_groups::{
    search_joint_placement, JointBounds, JointChoice, JointSearch, ListElementGroups,
    ListGroupPlacement,
};
use crate::phase::construction::run_construction_phase;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepControlPolicy};
use crate::stats::{CandidateTraceDisposition, CandidateTracePullToken, CandidateTraceSource};

struct UnitChoice<Sc> {
    unit: usize,
    regret: RegretValue<Sc>,
    choice: JointChoice<Sc>,
    token: Option<CandidateTracePullToken>,
}

/// Runs grouped regret insertion as one construction phase.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_regret_grouped<S, A, D, BestCb>(
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
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    run_construction_phase(
        solver_scope,
        0,
        "Regret-Insertion Construction",
        |phase_scope| {
            let solution = phase_scope.score_director().working_solution();
            let mut ordered = bound_unassigned.to_vec();
            ordered.sort_by_key(|entry| {
                (
                    access.construction_order_key(solution, &entry.element),
                    entry.source_index,
                )
            });
            let mut units =
                construction_units(access, solution, source_index, &ordered, groups, placement);
            if access.entity_count(solution) == 0 {
                units.clear();
            }
            while !units.is_empty() {
                if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                    break;
                }
                let mut best: Option<UnitChoice<S::Score>> = None;
                let mut interrupted = false;
                for (index, unit) in units.iter().enumerate() {
                    let Some(outcome) =
                        unit_regret(access, unit, bounds, control_policy, phase_scope)
                    else {
                        interrupted = true;
                        break;
                    };
                    let Some((regret, choice, token)) = outcome else {
                        continue;
                    };
                    let better = best.as_ref().is_none_or(|best| {
                        regret_choice_is_better(
                            regret,
                            choice.score,
                            best.regret,
                            best.choice.score,
                        )
                    });
                    let ignored = if better {
                        best.replace(UnitChoice {
                            unit: index,
                            regret,
                            choice,
                            token,
                        })
                        .and_then(|previous| previous.token)
                    } else {
                        token
                    };
                    if let Some(token) = ignored {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                }
                if interrupted {
                    if let Some(token) = best.and_then(|best| best.token) {
                        phase_scope.record_candidate_trace_disposition(
                            token,
                            CandidateTraceDisposition::ForagerIgnored,
                        );
                    }
                    break;
                }
                let Some(best) = best else {
                    tracing::warn!("No valid insertion found for remaining groups, stopping");
                    break;
                };
                let unit = units.remove(best.unit);
                commit_group(
                    access,
                    unit.elements(),
                    best.choice.entity,
                    &best.choice.positions,
                    best.choice.score,
                    best.token,
                    control_policy,
                    phase_scope,
                );
            }
            phase_scope.update_best_solution();
        },
    );
}

/// A unit's best placement and its regret; `None` when interrupted, and
/// `Some(None)` when no owner admits it.
#[allow(clippy::type_complexity)]
fn unit_regret<S, A, D, BestCb>(
    access: &A,
    unit: &ConstructionUnit<A::Element>,
    bounds: JointBounds,
    control_policy: StepControlPolicy,
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
) -> Option<
    Option<(
        RegretValue<S::Score>,
        JointChoice<S::Score>,
        Option<CandidateTracePullToken>,
    )>,
>
where
    S: PlanningSolution,
    A: RegretAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
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
            if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
                return None;
            }
            let (score, token) = evaluate_group_trial(
                access,
                &mut members,
                first_source,
                entity,
                positions,
                CandidateTraceSource::ListRegretInsertionTrial,
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
        JointSearch::Interrupted => {
            if let Some(token) = token {
                phase_scope.record_candidate_trace_disposition(
                    token,
                    CandidateTraceDisposition::ForagerIgnored,
                );
            }
            None
        }
        JointSearch::Done { best: None, .. } => Some(None),
        JointSearch::Done {
            best: Some(choice),
            second,
        } => {
            let regret = second.map_or(RegretValue::Forced, |second| {
                RegretValue::Finite(choice.score - second)
            });
            Some(Some((regret, choice, token)))
        }
    }
}
