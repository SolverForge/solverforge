use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{AssignmentMoveIntent, ScalarAssignmentMoveOptions};
use super::assignment_entity::{
    required_entities_by_scarcity, required_value_degrees, sort_values_by_required_scarcity,
};
use super::assignment_path::{assignment_move_for_entity_value, move_from_edits};
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) fn required_batch_move<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    options: ScalarAssignmentMoveOptions,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let entities =
        required_entities_by_scarcity(group, solution, state, options.value_candidate_limit);
    let value_degrees =
        required_value_degrees(group, solution, &entities, options.value_candidate_limit);
    let mut scalar_edits = Vec::new();
    let mut edited_entities = HashSet::new();
    for entity_index in entities {
        if state.current_value(entity_index).is_some() {
            continue;
        }
        let mut values =
            group.candidate_values(solution, entity_index, options.value_candidate_limit);
        sort_values_by_required_scarcity(
            group,
            solution,
            entity_index,
            &value_degrees,
            &mut values,
        );
        for value in values {
            let Some(mov) = assignment_move_for_entity_value(
                group,
                solution,
                state,
                entity_index,
                value,
                options,
                AssignmentMoveIntent::required(),
            ) else {
                continue;
            };
            if mov
                .edits()
                .iter()
                .any(|edit| edited_entities.contains(&edit.entity_index))
            {
                continue;
            }
            if mov.edits().len() != 1 {
                continue;
            }
            for edit in mov.edits() {
                state.set_value(group, solution, edit.entity_index, edit.to_value);
                edited_entities.insert(edit.entity_index);
                scalar_edits.push(group.edit(edit.entity_index, edit.to_value));
            }
            break;
        }
    }
    if scalar_edits.is_empty() {
        return None;
    }
    move_from_edits(group, solution, &scalar_edits, "scalar_assignment_required")
}
