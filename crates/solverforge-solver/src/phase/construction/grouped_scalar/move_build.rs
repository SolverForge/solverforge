use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use crate::builder::context::{ScalarAssignmentBinding, ScalarCandidate, ScalarGroupBinding};
use crate::heuristic::r#move::CompoundScalarMove;
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
        edits.push(member.compound_edit(edit.entity_index(), edit.to_value()));
    }

    Some(CompoundScalarMove::new(candidate.reason(), edits))
}

pub(super) fn compound_move_for_prevalidated_group_candidate<S>(
    group: &ScalarGroupBinding<S>,
    candidate: &ScalarCandidate<S>,
) -> CompoundScalarMove<S> {
    let edits = candidate
        .edits()
        .iter()
        .map(|edit| {
            let member = group
                .member_for_edit(edit)
                .expect("prevalidated grouped scalar candidate member must exist");
            member.compound_edit(edit.entity_index(), edit.to_value())
        })
        .collect();
    CompoundScalarMove::new(candidate.reason(), edits)
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
        compound_edits.push(
            group
                .target()
                .compound_edit(edit.entity_index(), edit.to_value()),
        );
    }

    Some(CompoundScalarMove::new(reason, compound_edits))
}
