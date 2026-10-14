use crate::builder::ScalarAssignmentBinding;
use crate::planning::ScalarEdit;
#[path = "assignment_overlay.rs"]
mod overlay;
#[path = "assignment_pressure.rs"]
mod pressure;

use super::assignment_path_overlay::AssignmentPathOverlay;
use overlay::OverlayScratch;

use solverforge_core::fast_hash::FastMap;

mod membership;

#[cfg(test)]
#[path = "tests/membership_keys.rs"]
mod membership_keys;

pub(crate) struct ScalarAssignmentState {
    values: Vec<Option<usize>>,
    capacity_keys: Vec<Option<usize>>,
    sequence_keys: Vec<Option<usize>>,
    required: Vec<bool>,
    occupancy: FastMap<usize, Vec<usize>>,
    assigned_by_value: FastMap<usize, Vec<usize>>,
    assigned_by_sequence: FastMap<(usize, usize), Vec<usize>>,
    /* Pooled scratch for the hot per-candidate feasibility path; each field
    serves one nesting level (take/restore, allocated once grown). */
    overlay: OverlayScratch,
    path_overlay: AssignmentPathOverlay,
}

pub(super) struct CapacityConflict {
    pub(super) key: usize,
    pub(super) occupants: Vec<usize>,
}

#[cfg(test)]
#[derive(Debug, Eq, PartialEq)]
pub(super) struct AssignmentIndexSnapshot {
    occupancy: Vec<(usize, Vec<usize>)>,
    assigned_by_value: Vec<(usize, Vec<usize>)>,
    assigned_by_sequence: Vec<((usize, usize), Vec<usize>)>,
}

impl ScalarAssignmentState {
    #[cfg(test)]
    pub(super) fn index_snapshot(&self) -> AssignmentIndexSnapshot {
        let mut occupancy = self
            .occupancy
            .iter()
            .map(|(key, entities)| (*key, entities.clone()))
            .collect::<Vec<_>>();
        occupancy.sort_by_key(|(key, _)| *key);
        let mut assigned_by_value = self
            .assigned_by_value
            .iter()
            .map(|(key, entities)| (*key, entities.clone()))
            .collect::<Vec<_>>();
        assigned_by_value.sort_by_key(|(key, _)| *key);
        let mut assigned_by_sequence = self
            .assigned_by_sequence
            .iter()
            .map(|(key, entities)| (*key, entities.clone()))
            .collect::<Vec<_>>();
        assigned_by_sequence.sort_by_key(|(key, _)| *key);
        AssignmentIndexSnapshot {
            occupancy,
            assigned_by_value,
            assigned_by_sequence,
        }
    }

    pub(super) fn new<S>(group: &ScalarAssignmentBinding<S>, solution: &S) -> Self {
        let entity_count = group.entity_count(solution);
        let mut state = Self {
            values: vec![None; entity_count],
            capacity_keys: vec![None; entity_count],
            sequence_keys: vec![None; entity_count],
            required: vec![false; entity_count],
            occupancy: FastMap::default(),
            assigned_by_value: FastMap::default(),
            assigned_by_sequence: FastMap::default(),
            overlay: OverlayScratch::default(),
            path_overlay: AssignmentPathOverlay::default(),
        };
        for entity in 0..entity_count {
            state.refresh_rows(
                group,
                solution,
                std::iter::once((entity, group.current_value(solution, entity))),
            );
        }
        state
    }

    pub(crate) fn row_count(&self) -> usize {
        self.values.len()
    }

    pub(super) fn is_required(&self, entity_index: usize) -> bool {
        self.required.get(entity_index).copied().unwrap_or(false)
    }

    pub(crate) fn current_value(&self, entity_index: usize) -> Option<usize> {
        self.values.get(entity_index).copied().flatten()
    }

    pub(super) fn recorded_sequence_key(&self, entity: usize) -> Option<usize> {
        self.sequence_keys.get(entity).copied().flatten()
    }

    pub(super) fn assigned_count(&self, value: usize) -> usize {
        self.assigned_by_value
            .get(&value)
            .map(Vec::len)
            .unwrap_or(0)
    }

    pub(super) fn take_path_overlay(&mut self) -> AssignmentPathOverlay {
        std::mem::take(&mut self.path_overlay)
    }

    pub(super) fn restore_path_overlay(&mut self, overlay: AssignmentPathOverlay) {
        self.path_overlay = overlay;
    }

    pub(super) fn refresh_trial_edits<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: &[ScalarEdit<S>],
    ) -> bool {
        // Final feasibility stays overlay-based on the committed baseline; the
        // caller restores the path overlay. Nothing here may mutate base rows:
        // augmentation reference math depends on feasibility and recorded keys
        // seeing the same baseline as candidate discovery.
        self.scalar_edits_feasible(group, solution, edits)
    }

    pub(super) fn capacity_occupants(&self, key: usize) -> &[usize] {
        self.occupancy.get(&key).map(Vec::as_slice).unwrap_or(&[])
    }

    pub(super) fn sequence_members(&self, value: usize, sequence: usize) -> &[usize] {
        self.assigned_by_sequence
            .get(&(value, sequence))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[cfg(test)]
    pub(super) fn set_value<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: Option<usize>,
    ) {
        self.values[entity] = value;
        self.occupancy.clear();
        self.assigned_by_value.clear();
        self.assigned_by_sequence.clear();
        for (entity, value) in self.values.iter().copied().enumerate() {
            self.capacity_keys[entity] =
                value.and_then(|value| group.capacity_key(solution, entity, value));
            self.sequence_keys[entity] =
                value.and_then(|value| group.sequence_key(solution, entity, value));
            let Some(value) = value else {
                continue;
            };
            self.assigned_by_value
                .entry(value)
                .or_default()
                .push(entity);
            if let Some(sequence) = self.sequence_keys[entity] {
                self.assigned_by_sequence
                    .entry((value, sequence))
                    .or_default()
                    .push(entity);
            }
            if let Some(key) = self.capacity_keys[entity] {
                self.occupancy.entry(key).or_default().push(entity);
            }
        }
    }

    #[cfg(test)]
    pub(super) fn assignment_allowed<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
    ) -> bool {
        if !group.value_is_legal(solution, entity, Some(value)) || !group.has_assignment_rule() {
            return group.value_is_legal(solution, entity, Some(value));
        }
        let Some(sequence) = group.sequence_key(solution, entity, value) else {
            return true;
        };
        adjacent_sequences(sequence).all(|adjacent| {
            self.sequence_members(value, adjacent)
                .iter()
                .copied()
                .filter(|neighbor| *neighbor != entity)
                .all(|neighbor| {
                    if adjacent < sequence {
                        group.assignment_edge_allowed(solution, neighbor, value, entity, value)
                    } else {
                        group.assignment_edge_allowed(solution, entity, value, neighbor, value)
                    }
                })
        })
    }

    #[cfg(test)]
    pub(super) fn blockers<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
        blockers: &mut Vec<usize>,
    ) -> Option<bool> {
        blockers.clear();
        if let Some(key) = group.capacity_key(solution, entity, value) {
            blockers.extend(
                self.capacity_occupants(key)
                    .iter()
                    .copied()
                    .filter(|occupant| *occupant != entity),
            );
        }
        let verdict = group.has_assignment_rule().then(|| {
            let allowed = self.assignment_allowed(group, solution, entity, value);
            if !allowed {
                if let Some(sequence) = group.sequence_key(solution, entity, value) {
                    for adjacent in adjacent_sequences(sequence) {
                        blockers.extend(
                            self.sequence_members(value, adjacent)
                                .iter()
                                .copied()
                                .filter(|blocker| *blocker != entity),
                        );
                    }
                }
            }
            allowed
        });
        blockers.sort_unstable();
        blockers.dedup();
        verdict
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
}

pub(super) fn adjacent_sequences(sequence: usize) -> impl Iterator<Item = usize> {
    [sequence.checked_sub(1), sequence.checked_add(1)]
        .into_iter()
        .flatten()
}

fn pair_was_checked(checked: &[(usize, usize)], left: usize, right: usize) -> bool {
    checked
        .iter()
        .any(|(checked_left, checked_right)| *checked_left == left && *checked_right == right)
}
