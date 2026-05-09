use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{ScalarAssignmentBinding, ScalarCandidate, ScalarGroupBinding};
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove};
use crate::planning::ScalarEdit;

pub(crate) fn compound_move_for_group_candidate<S>(
    group: &ScalarGroupBinding<S>,
    solution: &S,
    candidate: &ScalarCandidate<S>,
) -> Option<CompoundScalarMove<S>> {
    let mut edits = Vec::with_capacity(candidate.edits().len());
    for edit in candidate.edits() {
        let member = group.member_for_edit(edit)?;
        if !member.value_is_legal(solution, edit.entity_index(), edit.to_value()) {
            return None;
        }
        edits.push(CompoundScalarEdit {
            descriptor_index: member.descriptor_index,
            entity_index: edit.entity_index(),
            variable_index: member.variable_index,
            variable_name: member.variable_name,
            to_value: edit.to_value(),
            getter: member.getter,
            setter: member.setter,
            value_is_legal: None,
        });
    }

    Some(CompoundScalarMove::new(candidate.reason(), edits))
}

pub(super) fn compound_move_for_assignment_edits<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    edits: &[ScalarEdit<S>],
    reason: &'static str,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    if edits.is_empty() {
        return None;
    }

    let mut targets = HashSet::new();
    let mut compound_edits = Vec::with_capacity(edits.len());
    for edit in edits {
        if !targets.insert(edit.entity_index()) {
            return None;
        }
        if !group.value_is_legal(solution, edit.entity_index(), edit.to_value()) {
            return None;
        }
        compound_edits.push(CompoundScalarEdit {
            descriptor_index: group.target.descriptor_index,
            entity_index: edit.entity_index(),
            variable_index: group.target.variable_index,
            variable_name: group.target.variable_name,
            to_value: edit.to_value(),
            getter: group.target.getter,
            setter: group.target.setter,
            value_is_legal: None,
        });
    }

    Some(CompoundScalarMove::new(reason, compound_edits))
}
