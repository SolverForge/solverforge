use crate::builder::context::{ScalarCandidate, ScalarGroupBinding};
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove};

pub(super) fn compound_move_for_group_candidate<S>(
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
