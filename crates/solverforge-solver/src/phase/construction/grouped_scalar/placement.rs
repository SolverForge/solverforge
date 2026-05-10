use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{ScalarAssignmentBinding, ScalarCandidate, ScalarGroupBinding};
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove};
use crate::heuristic::selector::EntityReference;
use crate::phase::construction::{
    ConstructionGroupSlotId, ConstructionGroupSlotKey, ConstructionSlotId, ConstructionTarget,
    Placement,
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

pub(super) fn placement_for_group_candidate<S>(
    sequence: usize,
    candidate: &ScalarCandidate<S>,
    group_slot: ConstructionGroupSlotId,
    scalar_slots: Vec<ConstructionSlotId>,
    keep_current_legal: bool,
    mov: CompoundScalarMove<S>,
) -> Placement<S, CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let entity_ref = candidate
        .edits()
        .first()
        .map(|edit| EntityReference::new(edit.descriptor_index(), edit.entity_index()))
        .unwrap_or_else(|| EntityReference::new(0, sequence));
    Placement::new(entity_ref, vec![mov])
        .with_group_slot(group_slot)
        .with_scalar_slots(scalar_slots)
        .with_keep_current_legal(keep_current_legal)
}

pub(super) fn push_or_merge_placement<S>(
    placements: &mut Vec<Placement<S, CompoundScalarMove<S>>>,
    mut placement: Placement<S, CompoundScalarMove<S>>,
) where
    S: PlanningSolution,
{
    if let Some(existing) = placements.iter_mut().find(|existing| {
        existing.group_slot() == placement.group_slot()
            && existing.scalar_slots() == placement.scalar_slots()
    }) {
        existing.moves.append(&mut placement.moves);
        return;
    }
    placements.push(placement);
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

pub(super) fn assignment_target_binding<'a, S>(
    assignment: &ScalarAssignmentBinding<S>,
    scalar_bindings: &'a [ResolvedVariableBinding<S>],
) -> Option<&'a ResolvedVariableBinding<S>> {
    scalar_bindings.iter().find(|binding| {
        binding.descriptor_index == assignment.target.descriptor_index
            && binding.variable_index == assignment.target.variable_index
    })
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

pub(super) fn assignment_move_target<S>(
    target_binding: &ResolvedVariableBinding<S>,
    group_slot: &ConstructionGroupSlotId,
    mov: &CompoundScalarMove<S>,
) -> ConstructionTarget {
    let scalar_slots = mov
        .edits()
        .iter()
        .filter_map(|edit| {
            (edit.descriptor_index == target_binding.descriptor_index
                && edit.variable_index == target_binding.variable_index)
                .then(|| target_binding.slot_id(edit.entity_index))
        })
        .collect::<Vec<_>>();
    ConstructionTarget::new()
        .with_scalar_slots(scalar_slots)
        .with_group_slot(group_slot.clone())
}

pub(super) fn assignment_move_touches_completed_slot<S, IsCompleted>(
    mov: &CompoundScalarMove<S>,
    target_binding: &ResolvedVariableBinding<S>,
    descriptor_index: usize,
    is_completed: &mut IsCompleted,
) -> bool
where
    S: PlanningSolution,
    IsCompleted: FnMut(&Placement<S, CompoundScalarMove<S>>) -> bool,
{
    mov.edits().iter().any(|edit| {
        let slot_id = target_binding.slot_id(edit.entity_index);
        let placement = Placement::new(
            EntityReference::new(descriptor_index, edit.entity_index),
            Vec::new(),
        )
        .with_scalar_slots(vec![slot_id]);
        is_completed(&placement)
    })
}

pub(super) fn placement_entity_order_key<S>(
    placement: &Placement<S, CompoundScalarMove<S>>,
) -> Option<i64>
where
    S: PlanningSolution,
{
    placement.construction_entity_order_key()
}

pub(super) fn placement_sequence<S>(placement: &Placement<S, CompoundScalarMove<S>>) -> usize
where
    S: PlanningSolution,
{
    placement.entity_ref.entity_index
}

pub(crate) fn scalar_group_move_strength<S>(mov: &CompoundScalarMove<S>, _solution: &S) -> i64
where
    S: PlanningSolution,
{
    mov.construction_value_order_key().unwrap_or(0)
}

pub(super) fn never_completed<S>(_placement: &Placement<S, CompoundScalarMove<S>>) -> bool
where
    S: PlanningSolution,
{
    false
}
