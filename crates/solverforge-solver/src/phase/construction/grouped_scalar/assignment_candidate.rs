use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::{CompoundScalarEdit, CompoundScalarMove};
use crate::planning::ScalarEdit;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScalarAssignmentMoveOptions {
    pub(crate) value_candidate_limit: Option<usize>,
    pub(crate) max_moves: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_rematch_size: usize,
}

impl ScalarAssignmentMoveOptions {
    pub(crate) fn for_construction(
        limits: crate::builder::ScalarGroupLimits,
        value_candidate_limit: Option<usize>,
        group_candidate_limit: Option<usize>,
    ) -> Self {
        Self {
            value_candidate_limit: value_candidate_limit.or(limits.value_candidate_limit),
            max_moves: group_candidate_limit
                .or(limits.group_candidate_limit)
                .unwrap_or(usize::MAX),
            max_depth: limits.max_augmenting_depth.unwrap_or(3),
            max_rematch_size: limits.max_rematch_size.unwrap_or(4).max(2),
        }
    }

    pub(crate) fn for_selector(
        limits: crate::builder::ScalarGroupLimits,
        value_candidate_limit: Option<usize>,
        max_moves_per_step: usize,
    ) -> Self {
        Self {
            value_candidate_limit: value_candidate_limit.or(limits.value_candidate_limit),
            max_moves: max_moves_per_step,
            max_depth: limits.max_augmenting_depth.unwrap_or(3),
            max_rematch_size: limits.max_rematch_size.unwrap_or(4).max(2),
        }
    }
}

#[derive(Clone, Copy)]
struct AssignmentMoveIntent {
    allow_optional_displacement: bool,
    reason: &'static str,
}

impl AssignmentMoveIntent {
    const fn required() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_required",
        }
    }

    const fn optional() -> Self {
        Self {
            allow_optional_displacement: false,
            reason: "scalar_assignment_optional",
        }
    }

    const fn capacity_repair() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_capacity_repair",
        }
    }

    const fn reassignment() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_reassignment",
        }
    }
}

pub(crate) fn remaining_required_count<S>(group: &ScalarAssignmentBinding<S>, solution: &S) -> u64 {
    (0..group.entity_count(solution))
        .filter(|entity_index| {
            group.is_required(solution, *entity_index)
                && group.current_value(solution, *entity_index).is_none()
        })
        .fold(0_u64, |count, _| count.saturating_add(1))
}

pub(crate) fn required_assignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let state = ScalarAssignmentState::new(group, solution);
    ordered_entities(group, solution, |entity_index| {
        state.is_required(entity_index) && state.current_value(entity_index).is_none()
    })
    .into_iter()
    .flat_map(|entity_index| {
        assignment_moves_for_entity(
            group,
            solution,
            entity_index,
            options,
            AssignmentMoveIntent::required(),
        )
    })
    .take(options.max_moves)
    .collect()
}

pub(crate) fn optional_assignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let state = ScalarAssignmentState::new(group, solution);
    ordered_entities(group, solution, |entity_index| {
        !state.is_required(entity_index) && state.current_value(entity_index).is_none()
    })
    .into_iter()
    .flat_map(|entity_index| {
        assignment_moves_for_entity(
            group,
            solution,
            entity_index,
            options,
            AssignmentMoveIntent::optional(),
        )
    })
    .take(options.max_moves)
    .collect()
}

pub(crate) fn capacity_conflict_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let state = ScalarAssignmentState::new(group, solution);
    let mut moves = Vec::new();
    let mut seen_entities = HashSet::new();
    for conflict in state.capacity_conflicts(group, solution) {
        let mut movers = conflict.occupants;
        movers.rotate_left(1);
        for entity_index in movers {
            if moves.len() >= options.max_moves || !seen_entities.insert(entity_index) {
                continue;
            }
            if !state.is_required(entity_index) {
                let edits = [group.edit(entity_index, None)];
                if let Some(mov) =
                    move_from_edits(group, solution, &edits, "scalar_assignment_capacity_repair")
                {
                    moves.push(mov);
                }
                continue;
            }
            let repair_moves = assignment_moves_for_entity(
                group,
                solution,
                entity_index,
                options,
                AssignmentMoveIntent::capacity_repair(),
            );
            moves.extend(
                repair_moves
                    .into_iter()
                    .take(options.max_moves - moves.len()),
            );
        }
        if moves.len() >= options.max_moves {
            break;
        }
    }
    moves
}

pub(crate) fn reassignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let state = ScalarAssignmentState::new(group, solution);
    ordered_entities(group, solution, |entity_index| {
        state.current_value(entity_index).is_some()
    })
    .into_iter()
    .flat_map(|entity_index| {
        assignment_moves_for_entity(
            group,
            solution,
            entity_index,
            options,
            AssignmentMoveIntent::reassignment(),
        )
    })
    .take(options.max_moves)
    .collect()
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

    let mut moves = Vec::new();
    let mut seen = HashSet::new();
    for entities in by_sequence.values_mut() {
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

fn ordered_entities<S, F>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    mut predicate: F,
) -> Vec<usize>
where
    F: FnMut(usize) -> bool,
{
    let mut entities = (0..group.entity_count(solution))
        .filter(|entity_index| predicate(*entity_index))
        .collect::<Vec<_>>();
    entities.sort_by_key(|entity_index| {
        (
            group.entity_order_key(solution, *entity_index),
            *entity_index,
        )
    });
    entities
}

fn assignment_moves_for_entity<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    entity_index: usize,
    options: ScalarAssignmentMoveOptions,
    intent: AssignmentMoveIntent,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let values = group.candidate_values(solution, entity_index, options.value_candidate_limit);
    let mut moves = Vec::new();
    let mut state = ScalarAssignmentState::new(group, solution);
    let mut changes = Vec::new();
    let mut edits = Vec::new();
    let mut visiting = HashSet::new();
    let search = AugmentingPathSearch {
        group,
        solution,
        allow_optional_displacement: intent.allow_optional_displacement,
        value_candidate_limit: options.value_candidate_limit,
    };
    for value in values {
        if state.current_value(entity_index) == Some(value) {
            continue;
        }
        let change_checkpoint = changes.len();
        let edit_checkpoint = edits.len();
        visiting.clear();
        if search.assign(
            &mut state,
            AssignmentRequest {
                entity_index,
                value,
                depth: options.max_depth,
            },
            &mut visiting,
            &mut changes,
            &mut edits,
        ) {
            if let Some(mov) =
                move_from_edits(group, solution, &edits[edit_checkpoint..], intent.reason)
            {
                moves.push(mov);
            }
        }
        state.rollback(group, solution, &mut changes, change_checkpoint);
        edits.truncate(edit_checkpoint);
        if moves.len() >= options.max_moves {
            break;
        }
    }
    moves
}

#[derive(Clone, Copy)]
struct AssignmentRequest {
    entity_index: usize,
    value: usize,
    depth: usize,
}

struct AugmentingPathSearch<'a, S> {
    group: &'a ScalarAssignmentBinding<S>,
    solution: &'a S,
    allow_optional_displacement: bool,
    value_candidate_limit: Option<usize>,
}

impl<S> AugmentingPathSearch<'_, S> {
    fn assign(
        &self,
        state: &mut ScalarAssignmentState,
        assignment: AssignmentRequest,
        visiting: &mut HashSet<usize>,
        changes: &mut Vec<(usize, Option<usize>)>,
        edits: &mut Vec<ScalarEdit<S>>,
    ) -> bool {
        let entity_index = assignment.entity_index;
        let value = assignment.value;
        if !self
            .group
            .value_is_legal(self.solution, entity_index, Some(value))
        {
            return false;
        }
        if state.current_value(entity_index) == Some(value) {
            return true;
        }

        let mut blockers = state.blockers(self.group, self.solution, entity_index, value);
        if self.allow_optional_displacement {
            for blocker in blockers.iter().copied() {
                if state.is_required(blocker) {
                    continue;
                }
                state.set_value_recording(self.group, self.solution, blocker, None, changes);
                edits.push(self.group.edit(blocker, None));
            }
            blockers = state.blockers(self.group, self.solution, entity_index, value);
        }

        if blockers.is_empty() {
            state.set_value_recording(
                self.group,
                self.solution,
                entity_index,
                Some(value),
                changes,
            );
            edits.push(self.group.edit(entity_index, Some(value)));
            return true;
        }

        if assignment.depth == 0 || !visiting.insert(entity_index) {
            return false;
        }

        let Some(blocker) = blockers
            .into_iter()
            .find(|blocker| state.is_required(*blocker))
        else {
            visiting.remove(&entity_index);
            return false;
        };

        let alternatives =
            self.group
                .candidate_values(self.solution, blocker, self.value_candidate_limit);
        for alternative in alternatives {
            if state.current_value(blocker) == Some(alternative) {
                continue;
            }
            let change_checkpoint = changes.len();
            let edit_checkpoint = edits.len();
            if self.assign(
                state,
                AssignmentRequest {
                    entity_index: blocker,
                    value: alternative,
                    depth: assignment.depth - 1,
                },
                visiting,
                changes,
                edits,
            ) && state
                .blockers(self.group, self.solution, entity_index, value)
                .is_empty()
            {
                state.set_value_recording(
                    self.group,
                    self.solution,
                    entity_index,
                    Some(value),
                    changes,
                );
                edits.push(self.group.edit(entity_index, Some(value)));
                visiting.remove(&entity_index);
                return true;
            }
            state.rollback(self.group, self.solution, changes, change_checkpoint);
            edits.truncate(edit_checkpoint);
        }

        visiting.remove(&entity_index);
        false
    }
}

fn move_from_edits<S>(
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
