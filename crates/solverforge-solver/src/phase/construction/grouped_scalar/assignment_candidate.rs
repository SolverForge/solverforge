use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use super::assignment_path::{assignment_moves_for_entity, move_from_edits};
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ScalarAssignmentMoveOptions {
    pub(crate) value_candidate_limit: Option<usize>,
    pub(crate) max_moves: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_rematch_size: usize,
}

impl ScalarAssignmentMoveOptions {
    pub(crate) fn for_construction(limits: crate::builder::ScalarGroupLimits) -> Self {
        Self {
            value_candidate_limit: limits.value_candidate_limit,
            max_moves: limits.group_candidate_limit.unwrap_or(usize::MAX),
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
pub(super) struct AssignmentMoveIntent {
    pub(super) allow_optional_displacement: bool,
    pub(super) reason: &'static str,
}

impl AssignmentMoveIntent {
    pub(super) const fn required() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_required",
        }
    }

    pub(super) const fn optional() -> Self {
        Self {
            allow_optional_displacement: false,
            reason: "scalar_assignment_optional",
        }
    }

    pub(super) const fn capacity_repair() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_capacity_repair",
        }
    }

    pub(super) const fn reassignment() -> Self {
        Self {
            allow_optional_displacement: true,
            reason: "scalar_assignment_reassignment",
        }
    }
}

pub(crate) fn remaining_required_count<S>(group: &ScalarAssignmentBinding<S>, solution: &S) -> u64 {
    group.remaining_required_count(solution)
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

pub(super) fn ordered_entities<S, F>(
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
