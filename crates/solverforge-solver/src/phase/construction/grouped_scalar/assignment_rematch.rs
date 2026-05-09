use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(crate) fn rematch_assignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let mut state = ScalarAssignmentState::new(group, solution);
    let mut by_sequence: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
    for entity_index in 0..group.entity_count(solution) {
        let Some(value) = state.current_value(entity_index) else {
            continue;
        };
        by_sequence
            .entry(group.sequence_key(solution, entity_index, value))
            .or_default()
            .push(entity_index);
    }

    let mut sequence_keys = by_sequence.keys().copied().collect::<Vec<_>>();
    sequence_keys.sort_unstable();

    let mut moves = Vec::new();
    let mut seen = HashSet::new();
    for sequence_key in sequence_keys {
        let Some(entities) = by_sequence.get_mut(&sequence_key) else {
            continue;
        };
        entities.sort_by_key(|entity_index| {
            (
                group.position_key(solution, *entity_index),
                group.entity_order_key(solution, *entity_index),
                *entity_index,
            )
        });
        for left_pos in 0..entities.len() {
            let right_limit = (left_pos + options.max_rematch_size).min(entities.len());
            for right_pos in (left_pos + 1)..right_limit {
                if moves.len() >= options.max_moves {
                    return moves;
                }
                let left = entities[left_pos];
                let right = entities[right_pos];
                let Some(left_value) = state.current_value(left) else {
                    continue;
                };
                let Some(right_value) = state.current_value(right) else {
                    continue;
                };
                if left_value == right_value
                    || !group.value_is_legal(solution, left, Some(right_value))
                    || !group.value_is_legal(solution, right, Some(left_value))
                {
                    continue;
                }
                let normalized = if left < right {
                    (left, right, left_value, right_value)
                } else {
                    (right, left, right_value, left_value)
                };
                if !seen.insert(normalized) {
                    continue;
                }
                let edits = [(left, Some(right_value)), (right, Some(left_value))];
                if !state.capacity_feasible_after_edits(group, solution, &edits) {
                    continue;
                }
                let scalar_edits = [
                    group.edit(left, Some(right_value)),
                    group.edit(right, Some(left_value)),
                ];
                if let Some(mov) =
                    move_from_edits(group, solution, &scalar_edits, "scalar_assignment_rematch")
                {
                    moves.push(mov);
                }
            }
        }
    }
    moves
}
