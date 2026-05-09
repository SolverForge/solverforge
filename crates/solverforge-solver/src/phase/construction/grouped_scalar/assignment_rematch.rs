use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{rotate_entity_order, ScalarAssignmentMoveOptions};
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(crate) fn sequence_window_assignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    if group.position_key.is_none() || group.sequence_key.is_none() {
        return Vec::new();
    }

    let mut assigned = Vec::new();
    for entity_index in 0..group.entity_count(solution) {
        let Some(value) = group.current_value(solution, entity_index) else {
            continue;
        };
        let Some(position_key) = group.position_key(solution, entity_index) else {
            continue;
        };
        assigned.push((
            position_key,
            group.sequence_key(solution, entity_index, value),
            entity_index,
            value,
        ));
    }
    assigned.sort_unstable();
    if !assigned.is_empty() {
        let assigned_len = assigned.len();
        assigned.rotate_left(options.entity_offset % assigned_len);
    }

    let mut state = ScalarAssignmentState::new(group, solution);
    let mut moves = Vec::new();
    for left_pos in 0..assigned.len() {
        let right_limit = (left_pos + options.max_rematch_size).min(assigned.len());
        for right_pos in (left_pos + 1)..right_limit {
            if moves.len() >= options.max_moves {
                return moves;
            }

            let (_, _, left_entity, left_value) = assigned[left_pos];
            let (_, _, right_entity, right_value) = assigned[right_pos];
            if left_value == right_value
                || !group.value_is_legal(solution, left_entity, Some(right_value))
                || !group.value_is_legal(solution, right_entity, Some(left_value))
            {
                continue;
            }

            let edits = [
                (left_entity, Some(right_value)),
                (right_entity, Some(left_value)),
            ];
            if !state.capacity_feasible_after_edits(group, solution, &edits) {
                continue;
            }

            let scalar_edits = [
                group.edit(left_entity, Some(right_value)),
                group.edit(right_entity, Some(left_value)),
            ];
            if let Some(mov) = move_from_edits(
                group,
                solution,
                &scalar_edits,
                "scalar_assignment_sequence_window",
            ) {
                moves.push(mov);
            }
        }
    }
    moves
}

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
    if !sequence_keys.is_empty() {
        let sequence_key_count = sequence_keys.len();
        sequence_keys.rotate_left(options.entity_offset % sequence_key_count);
    }

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
        rotate_entity_order(entities, options.entity_offset);
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

pub(crate) fn paired_reassignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let mut state = ScalarAssignmentState::new(group, solution);
    let mut entities = (0..group.entity_count(solution))
        .filter(|entity_index| state.current_value(*entity_index).is_some())
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity_index| {
        (
            group.position_key(solution, *entity_index),
            group.entity_order_key(solution, *entity_index),
            *entity_index,
        )
    });
    rotate_entity_order(&mut entities, options.entity_offset);

    let mut moves = Vec::new();
    for (left_pos, &left) in entities.iter().enumerate() {
        let Some(left_current) = state.current_value(left) else {
            continue;
        };
        let left_values = group.candidate_values(solution, left, options.value_candidate_limit);
        for &right in entities.iter().skip(left_pos + 1) {
            let Some(right_current) = state.current_value(right) else {
                continue;
            };
            let right_values =
                group.candidate_values(solution, right, options.value_candidate_limit);
            for left_value in left_values.iter().copied() {
                if left_value == left_current
                    || !group.value_is_legal(solution, left, Some(left_value))
                {
                    continue;
                }
                for right_value in right_values.iter().copied() {
                    if moves.len() >= options.max_moves {
                        return moves;
                    }
                    if right_value == right_current
                        || !group.value_is_legal(solution, right, Some(right_value))
                    {
                        continue;
                    }
                    let edits = [(left, Some(left_value)), (right, Some(right_value))];
                    if !state.capacity_feasible_after_edits(group, solution, &edits) {
                        continue;
                    }
                    let scalar_edits = [
                        group.edit(left, Some(left_value)),
                        group.edit(right, Some(right_value)),
                    ];
                    if let Some(mov) = move_from_edits(
                        group,
                        solution,
                        &scalar_edits,
                        "scalar_assignment_pair_reassignment",
                    ) {
                        moves.push(mov);
                    }
                }
            }
        }
    }
    moves
}
