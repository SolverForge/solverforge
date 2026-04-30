use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::{ScalarGroupCandidate, ScalarGroupContext, ScalarGroupLimits};
use crate::descriptor_scalar::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarMove, Move};
use crate::scope::{PhaseScope, ProgressCallback};

use super::move_build::compound_move_for_group_candidate;
use crate::phase::construction::{
    ConstructionGroupSlotId, ConstructionGroupSlotKey, ConstructionSlotId,
};

pub(super) struct NormalizedGroupedCandidate<S>
where
    S: PlanningSolution,
{
    pub(super) sequence: usize,
    pub(super) group_slot: ConstructionGroupSlotId,
    pub(super) scalar_slots: Vec<ConstructionSlotId>,
    pub(super) keep_current_legal: bool,
    pub(super) entity_order_key: Option<i64>,
    pub(super) value_order_key: Option<i64>,
    pub(super) mov: CompoundScalarMove<S>,
}

pub(super) fn normalize_grouped_candidates<S, D, ProgressCb>(
    phase_scope: &PhaseScope<'_, '_, S, D, ProgressCb>,
    group_index: usize,
    group: &ScalarGroupContext<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    limits: ScalarGroupLimits,
    group_candidate_limit: Option<usize>,
) -> Vec<NormalizedGroupedCandidate<S>>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let solution = phase_scope.score_director().working_solution();
    let raw_candidates = (group.candidate_provider)(solution, limits);
    let total_limit = group_candidate_limit.unwrap_or(usize::MAX);
    if total_limit == 0 {
        return Vec::new();
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();

    for (sequence, candidate) in raw_candidates.into_iter().enumerate() {
        if candidate.edits.is_empty() || !seen.insert((candidate.reason, candidate.edits.clone())) {
            continue;
        }
        let Some((scalar_slots, keep_current_legal, has_unfinished_unassigned_slot)) =
            scalar_slots_for_candidate(phase_scope, group, scalar_bindings, solution, &candidate)
        else {
            continue;
        };
        if !has_unfinished_unassigned_slot {
            continue;
        }

        let group_slot = group_slot_id(group_index, &candidate, &scalar_slots);
        if phase_scope
            .solver_scope()
            .is_group_slot_completed(group_slot.clone())
        {
            continue;
        }

        let Some(mov) = compound_move_for_group_candidate(group, solution, &candidate) else {
            continue;
        };
        if !mov.is_doable(phase_scope.score_director()) {
            continue;
        }

        normalized.push(NormalizedGroupedCandidate {
            sequence,
            group_slot,
            scalar_slots,
            keep_current_legal,
            entity_order_key: candidate.construction_entity_order_key,
            value_order_key: candidate.construction_value_order_key,
            mov,
        });
        if normalized.len() >= total_limit {
            break;
        }
    }

    normalized
}

fn scalar_slots_for_candidate<S, D, ProgressCb>(
    phase_scope: &PhaseScope<'_, '_, S, D, ProgressCb>,
    group: &ScalarGroupContext<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    solution: &S,
    candidate: &ScalarGroupCandidate,
) -> Option<(Vec<ConstructionSlotId>, bool, bool)>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let mut targets = HashSet::new();
    let mut scalar_slots = Vec::with_capacity(candidate.edits.len());
    let mut keep_current_legal = true;
    let mut has_unfinished_unassigned_slot = false;

    for edit in &candidate.edits {
        if !targets.insert((edit.descriptor_index, edit.entity_index, edit.variable_name)) {
            return None;
        }
        let member = group.member_for_edit(edit)?;
        if !member.value_is_legal(solution, edit.entity_index, edit.to_value) {
            return None;
        }
        let binding = scalar_bindings.iter().find(|binding| {
            binding.descriptor_index == member.descriptor_index
                && binding.variable_index == member.variable_index
        })?;
        let slot = binding.slot_id(edit.entity_index);
        if phase_scope.solver_scope().is_scalar_slot_completed(slot) {
            return None;
        }
        if member.current_value(solution, edit.entity_index).is_none() {
            has_unfinished_unassigned_slot = true;
        }
        keep_current_legal &= member.allows_unassigned;
        scalar_slots.push(slot);
    }

    scalar_slots.sort_unstable();
    scalar_slots.dedup();
    Some((
        scalar_slots,
        keep_current_legal,
        has_unfinished_unassigned_slot,
    ))
}

fn group_slot_id(
    group_index: usize,
    candidate: &ScalarGroupCandidate,
    scalar_slots: &[ConstructionSlotId],
) -> ConstructionGroupSlotId {
    let key = candidate
        .construction_slot_key
        .map(ConstructionGroupSlotKey::Explicit)
        .unwrap_or_else(|| ConstructionGroupSlotKey::Targets(scalar_slots.to_vec()));
    ConstructionGroupSlotId::new(group_index, key)
}
