use std::collections::HashMap;

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{rotate_entity_order, ScalarAssignmentMoveOptions};
use super::assignment_entity::assigned_entities_by_position;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

enum PairWindowKind {
    SequenceWindow,
    Swap,
    Rematch,
    PairedReassignment,
}

pub(super) struct PairWindowCursor {
    kind: PairWindowKind,
    assigned: Vec<(i64, Option<usize>, usize, usize)>,
    entities: Vec<usize>,
    groups: Vec<Vec<usize>>,
    group_pos: usize,
    left_pos: usize,
    right_pos: usize,
    left_values: Vec<usize>,
    right_values: Vec<usize>,
    left_value_pos: usize,
    right_value_pos: usize,
    options: ScalarAssignmentMoveOptions,
}

impl PairWindowCursor {
    pub(super) fn sequence_window<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut assigned = Vec::new();
        if group.position_key.is_some() && group.sequence_key.is_some() {
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
        }
        Self::from_assigned(PairWindowKind::SequenceWindow, assigned, options)
    }

    pub(super) fn swap<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut entities = assigned_entities_by_position(group, solution, state);
        rotate_entity_order(&mut entities, options.entity_offset);
        Self::from_entities(PairWindowKind::Swap, entities, options)
    }

    pub(super) fn rematch<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
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
        let mut groups = Vec::with_capacity(sequence_keys.len());
        for sequence_key in sequence_keys {
            let Some(mut entities) = by_sequence.remove(&sequence_key) else {
                continue;
            };
            entities.sort_by_key(|entity_index| {
                (
                    group.position_key(solution, *entity_index),
                    group.entity_order_key(solution, *entity_index),
                    *entity_index,
                )
            });
            rotate_entity_order(&mut entities, options.entity_offset);
            groups.push(entities);
        }
        Self::from_groups(PairWindowKind::Rematch, groups, options)
    }

    pub(super) fn paired<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut entities = assigned_entities_by_position(group, solution, state);
        rotate_entity_order(&mut entities, options.entity_offset);
        Self::from_entities(PairWindowKind::PairedReassignment, entities, options)
    }

    fn from_assigned(
        kind: PairWindowKind,
        assigned: Vec<(i64, Option<usize>, usize, usize)>,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self {
            kind,
            assigned,
            entities: Vec::new(),
            groups: Vec::new(),
            group_pos: 0,
            left_pos: 0,
            right_pos: 1,
            left_values: Vec::new(),
            right_values: Vec::new(),
            left_value_pos: 0,
            right_value_pos: 0,
            options,
        }
    }

    fn from_entities(
        kind: PairWindowKind,
        entities: Vec<usize>,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self {
            kind,
            assigned: Vec::new(),
            entities,
            groups: Vec::new(),
            group_pos: 0,
            left_pos: 0,
            right_pos: 1,
            left_values: Vec::new(),
            right_values: Vec::new(),
            left_value_pos: 0,
            right_value_pos: 0,
            options,
        }
    }

    fn from_groups(
        kind: PairWindowKind,
        groups: Vec<Vec<usize>>,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self {
            kind,
            assigned: Vec::new(),
            entities: Vec::new(),
            groups,
            group_pos: 0,
            left_pos: 0,
            right_pos: 1,
            left_values: Vec::new(),
            right_values: Vec::new(),
            left_value_pos: 0,
            right_value_pos: 0,
            options,
        }
    }

    pub(super) fn next<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        match self.kind {
            PairWindowKind::SequenceWindow => self.next_sequence_window(group, solution, state),
            PairWindowKind::Swap => self.next_swap(group, solution, state),
            PairWindowKind::Rematch => self.next_rematch(group, solution, state),
            PairWindowKind::PairedReassignment => self.next_paired(group, solution, state),
        }
    }

    fn next_sequence_window<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.left_pos < self.assigned.len() {
            let right_limit =
                (self.left_pos + self.options.max_rematch_size).min(self.assigned.len());
            while self.right_pos < right_limit {
                let (_, _, left_entity, left_value) = self.assigned[self.left_pos];
                let (_, _, right_entity, right_value) = self.assigned[self.right_pos];
                self.right_pos += 1;
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
                if !state.assignment_feasible_after_edits(group, solution, &edits) {
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
                    return Some(mov);
                }
            }
            self.left_pos += 1;
            self.right_pos = self.left_pos + 1;
        }
        None
    }

    fn next_swap<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.left_pos < self.entities.len() {
            while self.right_pos < self.entities.len() {
                let left = self.entities[self.left_pos];
                let right = self.entities[self.right_pos];
                self.right_pos += 1;
                let Some(left_value) = state.current_value(left) else {
                    continue;
                };
                let Some(right_value) = state.current_value(right) else {
                    continue;
                };
                if left_value == right_value {
                    continue;
                }
                let edits = [(left, Some(right_value)), (right, Some(left_value))];
                if !state.assignment_feasible_after_edits(group, solution, &edits) {
                    continue;
                }
                let scalar_edits = [
                    group.edit(left, Some(right_value)),
                    group.edit(right, Some(left_value)),
                ];
                if let Some(mov) =
                    move_from_edits(group, solution, &scalar_edits, "scalar_assignment_swap")
                {
                    return Some(mov);
                }
            }
            self.left_pos += 1;
            self.right_pos = self.left_pos + 1;
        }
        None
    }

    fn next_rematch<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.group_pos < self.groups.len() {
            let entities = &self.groups[self.group_pos];
            while self.left_pos < entities.len() {
                let right_limit =
                    (self.left_pos + self.options.max_rematch_size).min(entities.len());
                while self.right_pos < right_limit {
                    let left = entities[self.left_pos];
                    let right = entities[self.right_pos];
                    self.right_pos += 1;
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
                    let edits = [(left, Some(right_value)), (right, Some(left_value))];
                    if !state.assignment_feasible_after_edits(group, solution, &edits) {
                        continue;
                    }
                    let scalar_edits = [
                        group.edit(left, Some(right_value)),
                        group.edit(right, Some(left_value)),
                    ];
                    if let Some(mov) =
                        move_from_edits(group, solution, &scalar_edits, "scalar_assignment_rematch")
                    {
                        return Some(mov);
                    }
                }
                self.left_pos += 1;
                self.right_pos = self.left_pos + 1;
            }
            self.group_pos += 1;
            self.left_pos = 0;
            self.right_pos = 1;
        }
        None
    }

    fn next_paired<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.left_pos < self.entities.len() {
            let right_limit =
                (self.left_pos + self.options.max_rematch_size).min(self.entities.len());
            if self.left_values.is_empty() {
                let left = self.entities[self.left_pos];
                self.left_values =
                    group.candidate_values(solution, left, self.options.value_candidate_limit);
                self.left_value_pos = 0;
            }
            while self.right_pos < right_limit {
                if self.right_values.is_empty() {
                    let right = self.entities[self.right_pos];
                    self.right_values =
                        group.candidate_values(solution, right, self.options.value_candidate_limit);
                    self.right_value_pos = 0;
                }
                let left = self.entities[self.left_pos];
                let right = self.entities[self.right_pos];
                let Some(left_current) = state.current_value(left) else {
                    self.advance_pair();
                    continue;
                };
                let Some(right_current) = state.current_value(right) else {
                    self.advance_pair();
                    continue;
                };
                while self.left_value_pos < self.left_values.len() {
                    let left_value = self.left_values[self.left_value_pos];
                    if left_value == left_current
                        || !group.value_is_legal(solution, left, Some(left_value))
                    {
                        self.left_value_pos += 1;
                        self.right_value_pos = 0;
                        continue;
                    }
                    while self.right_value_pos < self.right_values.len() {
                        let right_value = self.right_values[self.right_value_pos];
                        self.right_value_pos += 1;
                        if right_value == right_current
                            || !group.value_is_legal(solution, right, Some(right_value))
                        {
                            continue;
                        }
                        let edits = [(left, Some(left_value)), (right, Some(right_value))];
                        if !state.assignment_feasible_after_edits(group, solution, &edits) {
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
                            return Some(mov);
                        }
                    }
                    self.left_value_pos += 1;
                    self.right_value_pos = 0;
                }
                self.advance_pair();
            }
            self.left_pos += 1;
            self.right_pos = self.left_pos + 1;
            self.left_values.clear();
            self.left_value_pos = 0;
        }
        None
    }

    fn advance_pair(&mut self) {
        self.right_pos += 1;
        self.right_values.clear();
        self.left_value_pos = 0;
        self.right_value_pos = 0;
    }
}
