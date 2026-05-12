use std::collections::{HashMap, HashSet};

use crate::builder::ScalarAssignmentBinding;
use crate::planning::ScalarEdit;

use super::assignment_edge::{ForcedAssignment, SequenceEdge};
use super::assignment_index::{push_indexed_entity, remove_indexed_entity};

pub(super) struct ScalarAssignmentState {
    values: Vec<Option<usize>>,
    required: Vec<bool>,
    occupancy: HashMap<usize, Vec<usize>>,
    assigned_by_value: HashMap<usize, Vec<usize>>,
    assigned_by_sequence: HashMap<(usize, usize), Vec<usize>>,
}

pub(super) struct CapacityConflict {
    pub(super) key: usize,
    pub(super) occupants: Vec<usize>,
}

impl ScalarAssignmentState {
    pub(super) fn new<S>(group: &ScalarAssignmentBinding<S>, solution: &S) -> Self {
        let entity_count = group.entity_count(solution);
        let mut values = Vec::with_capacity(entity_count);
        let mut required = Vec::with_capacity(entity_count);
        let mut occupancy: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut assigned_by_value: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut assigned_by_sequence: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        for entity_index in 0..entity_count {
            let value = group.current_value(solution, entity_index);
            values.push(value);
            required.push(group.is_required(solution, entity_index));
            if let Some(value) = value {
                assigned_by_value
                    .entry(value)
                    .or_default()
                    .push(entity_index);
                if let Some(sequence_key) = group.sequence_key(solution, entity_index, value) {
                    assigned_by_sequence
                        .entry((value, sequence_key))
                        .or_default()
                        .push(entity_index);
                }
                if let Some(key) = group.capacity_key(solution, entity_index, value) {
                    occupancy.entry(key).or_default().push(entity_index);
                }
            }
        }
        Self {
            values,
            required,
            occupancy,
            assigned_by_value,
            assigned_by_sequence,
        }
    }

    pub(super) fn is_required(&self, entity_index: usize) -> bool {
        self.required.get(entity_index).copied().unwrap_or(false)
    }

    pub(super) fn current_value(&self, entity_index: usize) -> Option<usize> {
        self.values.get(entity_index).copied().flatten()
    }

    pub(super) fn assigned_count(&self, value: usize) -> usize {
        self.assigned_by_value
            .get(&value)
            .map(Vec::len)
            .unwrap_or(0)
    }

    pub(super) fn sort_entities_by_current_value_pressure<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entities: &mut [usize],
    ) {
        entities.sort_by_key(|entity_index| {
            let Some(value) = self.current_value(*entity_index) else {
                return (
                    usize::MAX,
                    usize::MAX,
                    group.entity_order_key(solution, *entity_index),
                    *entity_index,
                );
            };
            let load = self.assigned_count(value);
            let run = self.sequence_run_len(group, solution, *entity_index, value);
            (
                usize::MAX - run,
                usize::MAX - load,
                group.entity_order_key(solution, *entity_index),
                *entity_index,
            )
        });
    }

    pub(super) fn sort_values_by_target_pressure<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        values: &mut [usize],
    ) {
        values.sort_by_key(|value| {
            (
                self.target_sequence_neighbor_count(group, solution, entity_index, *value),
                self.assigned_count(*value),
                group.value_order_key(solution, entity_index, *value),
                *value,
            )
        });
    }

    fn sequence_run_len<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> usize {
        let Some(sequence_key) = group.sequence_key(solution, entity_index, value) else {
            return 0;
        };
        let mut len = 1;
        let mut previous = sequence_key;
        while let Some(next) = previous.checked_sub(1) {
            if !self.sequence_has_value(value, next) {
                break;
            }
            len += 1;
            previous = next;
        }
        let mut next = sequence_key;
        while let Some(sequence) = next.checked_add(1) {
            if !self.sequence_has_value(value, sequence) {
                break;
            }
            len += 1;
            next = sequence;
        }
        len
    }

    fn target_sequence_neighbor_count<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> usize {
        let Some(sequence_key) = group.sequence_key(solution, entity_index, value) else {
            return 0;
        };
        adjacent_sequences(sequence_key)
            .filter(|sequence| self.sequence_has_value(value, *sequence))
            .count()
    }

    fn sequence_has_value(&self, value: usize, sequence_key: usize) -> bool {
        self.assigned_by_sequence
            .get(&(value, sequence_key))
            .is_some_and(|entities| !entities.is_empty())
    }

    pub(super) fn set_value<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: Option<usize>,
    ) {
        self.remove_current(group, solution, entity_index);
        if let Some(value) = value {
            self.insert_current(group, solution, entity_index, value);
        }
    }

    pub(super) fn set_value_recording<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: Option<usize>,
        changes: &mut Vec<(usize, Option<usize>)>,
    ) {
        let previous = self.current_value(entity_index);
        if previous == value {
            return;
        }
        changes.push((entity_index, previous));
        self.set_value(group, solution, entity_index, value);
    }

    pub(super) fn rollback<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        changes: &mut Vec<(usize, Option<usize>)>,
        checkpoint: usize,
    ) {
        while changes.len() > checkpoint {
            let Some((entity_index, previous)) = changes.pop() else {
                break;
            };
            self.set_value(group, solution, entity_index, previous);
        }
    }

    pub(super) fn assignment_allowed<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> bool {
        group.value_is_legal(solution, entity_index, Some(value))
            && self.assignment_rule_allows_value(group, solution, entity_index, value)
    }

    pub(super) fn blockers<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> Vec<usize> {
        let mut blockers = Vec::new();
        let Some(key) = group.capacity_key(solution, entity_index, value) else {
            self.push_assignment_rule_blockers(group, solution, entity_index, value, &mut blockers);
            blockers.sort_unstable();
            blockers.dedup();
            return blockers;
        };
        if let Some(entities) = self.occupancy.get(&key) {
            blockers.extend(
                entities
                    .iter()
                    .copied()
                    .filter(|occupant| *occupant != entity_index),
            );
        }
        self.push_assignment_rule_blockers(group, solution, entity_index, value, &mut blockers);
        blockers.sort_unstable();
        blockers.dedup();
        blockers
    }

    pub(super) fn capacity_conflicts<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
    ) -> Vec<CapacityConflict> {
        let mut conflicts = self
            .occupancy
            .iter()
            .filter_map(|(key, occupants)| {
                if occupants.len() <= 1 {
                    return None;
                }
                let mut ordered_occupants = Vec::with_capacity(occupants.len());
                ordered_occupants.extend(occupants.iter().copied());
                ordered_occupants.sort_by_key(|entity_index| {
                    self.occupant_order_key(group, solution, *entity_index)
                });
                Some(CapacityConflict {
                    key: *key,
                    occupants: ordered_occupants,
                })
            })
            .collect::<Vec<_>>();
        conflicts.sort_by_key(|conflict| {
            let first_occupant = conflict.occupants[0];
            (
                self.occupant_order_key(group, solution, first_occupant),
                conflict.key,
            )
        });
        conflicts
    }

    pub(super) fn assignment_feasible_after_edits<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: &[(usize, Option<usize>)],
    ) -> bool {
        let mut changes = Vec::with_capacity(edits.len());
        let mut touched_capacity_keys = HashSet::new();
        let mut edited = Vec::with_capacity(edits.len());
        for (entity_index, value) in edits {
            if *entity_index >= self.values.len()
                || !group.value_is_legal(solution, *entity_index, *value)
            {
                self.rollback(group, solution, &mut changes, 0);
                return false;
            }
            let previous = self.current_value(*entity_index);
            if let Some(previous) = previous {
                if let Some(key) = group.capacity_key(solution, *entity_index, previous) {
                    touched_capacity_keys.insert(key);
                }
            }
            if let Some(value) = *value {
                if let Some(key) = group.capacity_key(solution, *entity_index, value) {
                    touched_capacity_keys.insert(key);
                }
            }
            edited.push((*entity_index, previous, *value));
            self.set_value_recording(group, solution, *entity_index, *value, &mut changes);
        }
        let capacity_feasible = touched_capacity_keys.iter().all(|key| {
            self.occupancy
                .get(key)
                .is_none_or(|occupants| occupants.len() <= 1)
        });
        let rule_feasible =
            capacity_feasible && self.assignment_rule_edges_feasible(group, solution, &edited);
        let feasible = capacity_feasible && rule_feasible;
        self.rollback(group, solution, &mut changes, 0);
        feasible
    }

    pub(super) fn scalar_edits_feasible<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: &[ScalarEdit<S>],
    ) -> bool {
        let edits = edits
            .iter()
            .map(|edit| (edit.entity_index(), edit.to_value()))
            .collect::<Vec<_>>();
        self.assignment_feasible_after_edits(group, solution, &edits)
    }

    fn assignment_rule_allows_value<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> bool {
        if group.assignment_rule.is_none() {
            return true;
        }

        let Some(sequence_key) = group.sequence_key(solution, entity_index, value) else {
            return true;
        };
        let mut checked = HashSet::new();
        let forced = ForcedAssignment {
            entity_index,
            value,
        };
        if let Some(previous_sequence) = sequence_key.checked_sub(1) {
            if !self.assignment_rule_allows_sequence_edge(
                group,
                solution,
                SequenceEdge::new(value, previous_sequence, sequence_key).with_forced_right(forced),
                &mut checked,
            ) {
                return false;
            }
        }
        if let Some(next_sequence) = sequence_key.checked_add(1) {
            if !self.assignment_rule_allows_sequence_edge(
                group,
                solution,
                SequenceEdge::new(value, sequence_key, next_sequence).with_forced_left(forced),
                &mut checked,
            ) {
                return false;
            }
        }
        true
    }

    fn assignment_rule_edges_feasible<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edited: &[(usize, Option<usize>, Option<usize>)],
    ) -> bool {
        if group.assignment_rule.is_none() {
            return true;
        }

        let mut checked = HashSet::new();
        for (entity_index, previous, next) in edited {
            for value in [*previous, *next].into_iter().flatten() {
                let Some(sequence_key) = group.sequence_key(solution, *entity_index, value) else {
                    continue;
                };
                if let Some(previous_sequence) = sequence_key.checked_sub(1) {
                    if !self.assignment_rule_allows_sequence_edge(
                        group,
                        solution,
                        SequenceEdge::new(value, previous_sequence, sequence_key),
                        &mut checked,
                    ) {
                        return false;
                    }
                }
                if let Some(next_sequence) = sequence_key.checked_add(1) {
                    if !self.assignment_rule_allows_sequence_edge(
                        group,
                        solution,
                        SequenceEdge::new(value, sequence_key, next_sequence),
                        &mut checked,
                    ) {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn assignment_rule_allows_sequence_edge<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edge: SequenceEdge,
        checked: &mut HashSet<(usize, usize)>,
    ) -> bool {
        let mut left_scratch = [0usize; 1];
        let mut right_scratch = [0usize; 1];
        let left_entities = match edge.forced_left {
            Some(forced) if forced.value == edge.value => {
                left_scratch[0] = forced.entity_index;
                &left_scratch[..]
            }
            Some(_) => &[][..],
            None => self
                .assigned_by_sequence
                .get(&(edge.value, edge.left_sequence))
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        };
        let right_entities = match edge.forced_right {
            Some(forced) if forced.value == edge.value => {
                right_scratch[0] = forced.entity_index;
                &right_scratch[..]
            }
            Some(_) => &[][..],
            None => self
                .assigned_by_sequence
                .get(&(edge.value, edge.right_sequence))
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        };

        for left in left_entities {
            for right in right_entities {
                if left == right || !checked.insert((*left, *right)) {
                    continue;
                }
                if !group.assignment_edge_allowed(solution, *left, edge.value, *right, edge.value) {
                    return false;
                }
            }
        }
        true
    }

    fn remove_current<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
    ) {
        let Some(value) = self.current_value(entity_index) else {
            return;
        };
        if let Some(key) = group.capacity_key(solution, entity_index, value) {
            if let Some(entities) = self.occupancy.get_mut(&key) {
                entities.retain(|occupant| *occupant != entity_index);
                if entities.is_empty() {
                    self.occupancy.remove(&key);
                }
            }
        }
        remove_indexed_entity(&mut self.assigned_by_value, value, entity_index);
        if let Some(sequence_key) = group.sequence_key(solution, entity_index, value) {
            remove_indexed_entity(
                &mut self.assigned_by_sequence,
                (value, sequence_key),
                entity_index,
            );
        }
        self.values[entity_index] = None;
    }

    fn insert_current<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) {
        self.values[entity_index] = Some(value);
        push_indexed_entity(&mut self.assigned_by_value, value, entity_index);
        if let Some(sequence_key) = group.sequence_key(solution, entity_index, value) {
            push_indexed_entity(
                &mut self.assigned_by_sequence,
                (value, sequence_key),
                entity_index,
            );
        }
        if let Some(key) = group.capacity_key(solution, entity_index, value) {
            push_indexed_entity(&mut self.occupancy, key, entity_index);
        }
    }

    fn occupant_order_key<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
    ) -> (bool, Option<i64>, usize) {
        (
            !self.is_required(entity_index),
            group.entity_order_key(solution, entity_index),
            entity_index,
        )
    }

    fn push_assignment_rule_blockers<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
        blockers: &mut Vec<usize>,
    ) {
        if group.assignment_rule.is_none()
            || self.assignment_allowed(group, solution, entity_index, value)
        {
            return;
        }

        let Some(target_sequence) = group.sequence_key(solution, entity_index, value) else {
            return;
        };
        for sequence_key in adjacent_sequences(target_sequence) {
            let Some(entities) = self.assigned_by_sequence.get(&(value, sequence_key)) else {
                continue;
            };
            for blocker in entities {
                if *blocker != entity_index {
                    blockers.push(*blocker);
                }
            }
        }
    }
}

fn adjacent_sequences(sequence_key: usize) -> impl Iterator<Item = usize> {
    [sequence_key.checked_sub(1), sequence_key.checked_add(1)]
        .into_iter()
        .flatten()
}
