use std::collections::HashSet;
use std::time::Instant;

use solverforge_config::{
    ConstructionHeuristicConfig, ConstructionHeuristicType, ConstructionObligation,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::builder::context::{ScalarGroupCandidate, ScalarGroupContext, ScalarGroupLimits};
use crate::descriptor_scalar::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove, Move};
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepScope};

use super::decision::keep_current_allowed;
use super::evaluation::evaluate_trial_move;
use super::{ConstructionGroupSlotId, ConstructionSlotId};

pub(crate) fn solve_grouped_scalar_construction<S, D, ProgressCb>(
    config: Option<&ConstructionHeuristicConfig>,
    group_index: usize,
    group: &ScalarGroupContext<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let construction_type = config
        .map(|cfg| cfg.construction_heuristic_type)
        .unwrap_or(ConstructionHeuristicType::FirstFit);
    let construction_obligation = config
        .map(|cfg| cfg.construction_obligation)
        .unwrap_or_default();
    let value_candidate_limit =
        config.and_then(|cfg| cfg.group_candidate_limit.or(cfg.value_candidate_limit));
    let limits = ScalarGroupLimits {
        value_candidate_limit,
        max_moves_per_step: None,
    };

    let mut phase_scope =
        PhaseScope::with_phase_type(solver_scope, 0, "Grouped Scalar Construction");
    let mut ran_step = false;

    loop {
        if phase_scope
            .solver_scope_mut()
            .should_terminate_construction()
        {
            break;
        }

        let Some(selection) = select_grouped_candidate(
            &mut phase_scope,
            group_index,
            group,
            scalar_bindings,
            limits,
            construction_type,
            construction_obligation,
        ) else {
            break;
        };

        ran_step = true;
        match selection {
            GroupedSelection::Commit {
                group_slot,
                scalar_slots,
                mov,
                score,
            } => {
                let mut step_scope = StepScope::new(&mut phase_scope);
                step_scope.phase_scope_mut().record_move_accepted();
                step_scope.apply_committed_move(&mov);
                step_scope.phase_scope_mut().record_move_applied();
                step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .mark_group_slot_completed(group_slot);
                for slot in scalar_slots {
                    step_scope
                        .phase_scope_mut()
                        .solver_scope_mut()
                        .mark_scalar_slot_completed(slot);
                }
                step_scope
                    .phase_scope_mut()
                    .record_construction_slot_assigned();
                step_scope.set_step_score(score);
                step_scope.complete();
            }
            GroupedSelection::CompleteOnly { group_slot, kept } => {
                let mut step_scope = StepScope::new(&mut phase_scope);
                step_scope
                    .phase_scope_mut()
                    .solver_scope_mut()
                    .mark_group_slot_completed(group_slot);
                if kept {
                    step_scope.phase_scope_mut().record_construction_slot_kept();
                } else {
                    step_scope
                        .phase_scope_mut()
                        .record_construction_slot_no_doable();
                }
                let score = step_scope.calculate_score();
                step_scope.set_step_score(score);
                step_scope.complete();
            }
        }
    }

    if ran_step {
        phase_scope.update_best_solution();
    }

    ran_step
}

enum GroupedSelection<S>
where
    S: PlanningSolution,
{
    Commit {
        group_slot: ConstructionGroupSlotId,
        scalar_slots: Vec<ConstructionSlotId>,
        mov: CompoundScalarMove<S>,
        score: S::Score,
    },
    CompleteOnly {
        group_slot: ConstructionGroupSlotId,
        kept: bool,
    },
}

#[allow(clippy::too_many_arguments)]
fn select_grouped_candidate<S, D, ProgressCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, ProgressCb>,
    group_index: usize,
    group: &ScalarGroupContext<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    limits: ScalarGroupLimits,
    construction_type: ConstructionHeuristicType,
    construction_obligation: ConstructionObligation,
) -> Option<GroupedSelection<S>>
where
    S: PlanningSolution + 'static,
    S::Score: Score + Copy,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let solution = phase_scope.score_director().working_solution();
    let candidates = (group.candidate_provider)(solution, limits);
    if candidates.is_empty() {
        return None;
    }

    let mut seen = HashSet::new();
    let mut first_unfinished_slot = None;
    let mut best: Option<(
        ConstructionGroupSlotId,
        Vec<ConstructionSlotId>,
        CompoundScalarMove<S>,
        S::Score,
    )> = None;
    let mut first_doable = None;

    for candidate in candidates {
        if candidate.edits.is_empty() || !seen.insert(candidate.clone()) {
            continue;
        }
        let group_slot = ConstructionGroupSlotId::new(group_index, group_slot_key(&candidate));
        if phase_scope
            .solver_scope()
            .is_group_slot_completed(group_slot)
        {
            continue;
        }
        if first_unfinished_slot.is_none() {
            first_unfinished_slot = Some(group_slot);
        }

        let Some((mov, scalar_slots, keep_current_legal)) = move_for_candidate(
            group,
            scalar_bindings,
            phase_scope.score_director().working_solution(),
            candidate,
        ) else {
            continue;
        };
        if !mov.is_doable(phase_scope.score_director()) {
            continue;
        }

        let generation_started = Instant::now();
        phase_scope.record_generated_move(generation_started.elapsed());
        let evaluation_started = Instant::now();
        let score = evaluate_trial_move(phase_scope.score_director_mut(), &mov);
        phase_scope.record_score_calculation();
        phase_scope.record_evaluated_move(evaluation_started.elapsed());

        let baseline_score = keep_current_allowed(keep_current_legal, construction_obligation)
            .then(|| phase_scope.calculate_score());
        if let Some(baseline_score) = baseline_score {
            if score <= baseline_score {
                return Some(GroupedSelection::CompleteOnly {
                    group_slot,
                    kept: construction_obligation == ConstructionObligation::PreserveUnassigned,
                });
            }
        }

        let entry = (group_slot, scalar_slots, mov, score);
        if construction_type == ConstructionHeuristicType::FirstFit {
            first_doable = Some(entry);
            break;
        }
        let should_replace = best
            .as_ref()
            .is_none_or(|(_, _, _, best_score)| score > *best_score);
        if should_replace {
            best = Some(entry);
        }
    }

    first_doable
        .or(best)
        .map(
            |(group_slot, scalar_slots, mov, score)| GroupedSelection::Commit {
                group_slot,
                scalar_slots,
                mov,
                score,
            },
        )
        .or_else(|| {
            first_unfinished_slot.map(|group_slot| GroupedSelection::CompleteOnly {
                group_slot,
                kept: false,
            })
        })
}

fn group_slot_key(candidate: &ScalarGroupCandidate) -> usize {
    candidate
        .edits
        .iter()
        .map(|edit| edit.entity_index)
        .min()
        .unwrap_or_default()
}

fn move_for_candidate<S>(
    group: &ScalarGroupContext<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    solution: &S,
    candidate: ScalarGroupCandidate,
) -> Option<(CompoundScalarMove<S>, Vec<ConstructionSlotId>, bool)>
where
    S: PlanningSolution + 'static,
{
    let mut targets = HashSet::new();
    let mut edits = Vec::with_capacity(candidate.edits.len());
    let mut scalar_slots = Vec::new();
    let mut keep_current_legal = true;
    for edit in candidate.edits {
        if !targets.insert((edit.descriptor_index, edit.entity_index, edit.variable_name)) {
            return None;
        }
        let member = group.member_for_edit(&edit)?;
        if !member.value_is_legal(solution, edit.entity_index, edit.to_value) {
            return None;
        }
        keep_current_legal &= member.allows_unassigned;
        if let Some(binding) = scalar_bindings.iter().find(|binding| {
            binding.descriptor_index == member.descriptor_index
                && binding.variable_index == member.variable_index
        }) {
            scalar_slots.push(binding.slot_id(edit.entity_index));
        }
        edits.push(CompoundScalarEdit {
            descriptor_index: member.descriptor_index,
            entity_index: edit.entity_index,
            variable_index: member.variable_index,
            variable_name: member.variable_name,
            to_value: edit.to_value,
            getter: member.getter,
            setter: member.setter,
            value_is_legal: None,
        });
    }

    Some((
        CompoundScalarMove::new(candidate.reason, edits),
        scalar_slots,
        keep_current_legal,
    ))
}
