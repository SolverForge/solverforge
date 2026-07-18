use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{ScalarCandidate, ScalarGroupBinding};
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove};
use crate::phase::construction::{
    ConstructionGroupSlotId, ConstructionGroupSlotKey, ConstructionSlotId, ConstructionTarget,
};

pub(super) fn scalar_slots_for_candidate<S>(
    group: &ScalarGroupBinding<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    solution: &S,
    candidate: &ScalarCandidate<S>,
) -> Option<(Vec<ConstructionSlotId>, bool, bool)>
where
    S: PlanningSolution,
{
    let mut targets = std::collections::HashSet::new();
    let mut scalar_slots = Vec::with_capacity(candidate.edits().len());
    let mut keep_current_legal = true;
    let mut has_unfinished_unassigned_slot = false;

    for edit in candidate.edits() {
        if !targets.insert((
            edit.descriptor_index(),
            edit.entity_index(),
            edit.variable_name(),
        )) {
            return None;
        }
        let member = group.member_for_edit(edit)?;
        if !member.value_is_legal(solution, edit.entity_index(), edit.to_value()) {
            return None;
        }
        let binding = scalar_bindings.iter().find(|binding| {
            binding.descriptor_index == member.descriptor_index
                && binding.variable_index == member.variable_index
        })?;
        let slot = binding.slot_id(edit.entity_index());
        if member
            .current_value(solution, edit.entity_index())
            .is_none()
        {
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

pub(super) fn group_slot_id<S>(
    group_index: usize,
    candidate: &ScalarCandidate<S>,
    scalar_slots: &[ConstructionSlotId],
) -> ConstructionGroupSlotId {
    let key = candidate
        .construction_slot_key()
        .map(ConstructionGroupSlotKey::Explicit)
        .unwrap_or_else(|| ConstructionGroupSlotKey::Targets(scalar_slots.to_vec()));
    ConstructionGroupSlotId::new(group_index, key)
}

pub(super) fn assignment_group_slot(
    group_index: usize,
    entity_index: usize,
) -> ConstructionGroupSlotId {
    ConstructionGroupSlotId::new(
        group_index,
        ConstructionGroupSlotKey::Explicit(entity_index),
    )
}

pub(super) fn principal_assignment_edit<S>(
    mov: &CompoundScalarMove<S>,
    entity_index: usize,
) -> Option<&CompoundScalarEdit<S>> {
    mov.edits()
        .iter()
        .find(|edit| edit.entity_index == entity_index && edit.to_value.is_some())
}

pub(super) fn assignment_move_target(group_slot: &ConstructionGroupSlotId) -> ConstructionTarget {
    ConstructionTarget::new().with_group_slot(group_slot.clone())
}

pub(crate) fn scalar_group_move_strength<S>(mov: &CompoundScalarMove<S>, _solution: &S) -> i64
where
    S: PlanningSolution,
{
    mov.construction_value_order_key().unwrap_or(0)
}
