use std::collections::HashMap;

use crate::builder::CoverageGroupBinding;

pub(super) struct CoverageState {
    values: Vec<Option<usize>>,
    required: Vec<bool>,
    occupancy: HashMap<usize, Vec<usize>>,
}

pub(super) struct CapacityConflict {
    pub(super) key: usize,
    pub(super) occupants: Vec<usize>,
}

impl CoverageState {
    pub(super) fn new<S>(group: &CoverageGroupBinding<S>, solution: &S) -> Self {
        let entity_count = group.entity_count(solution);
        let mut values = Vec::with_capacity(entity_count);
        let mut required = Vec::with_capacity(entity_count);
        let mut occupancy: HashMap<usize, Vec<usize>> = HashMap::new();
        for entity_index in 0..entity_count {
            let value = group.current_value(solution, entity_index);
            values.push(value);
            required.push(group.is_required(solution, entity_index));
            if let Some(value) = value {
                if let Some(key) = group.capacity_key(solution, entity_index, value) {
                    occupancy.entry(key).or_default().push(entity_index);
                }
            }
        }
        Self {
            values,
            required,
            occupancy,
        }
    }

    pub(super) fn is_required(&self, entity_index: usize) -> bool {
        self.required.get(entity_index).copied().unwrap_or(false)
    }

    pub(super) fn current_value(&self, entity_index: usize) -> Option<usize> {
        self.values.get(entity_index).copied().flatten()
    }

    pub(super) fn set_value<S>(
        &mut self,
        group: &CoverageGroupBinding<S>,
        solution: &S,
        entity_index: usize,
        value: Option<usize>,
    ) {
        self.remove_current(group, solution, entity_index);
        self.values[entity_index] = value;
        if let Some(value) = value {
            if let Some(key) = group.capacity_key(solution, entity_index, value) {
                let entities = self.occupancy.entry(key).or_default();
                if !entities.contains(&entity_index) {
                    entities.push(entity_index);
                }
            }
        }
    }

    pub(super) fn set_value_recording<S>(
        &mut self,
        group: &CoverageGroupBinding<S>,
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
        group: &CoverageGroupBinding<S>,
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

    pub(super) fn blockers<S>(
        &self,
        group: &CoverageGroupBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> Vec<usize> {
        let Some(key) = group.capacity_key(solution, entity_index, value) else {
            return Vec::new();
        };
        self.occupancy
            .get(&key)
            .into_iter()
            .flat_map(|entities| entities.iter().copied())
            .filter(|occupant| *occupant != entity_index)
            .collect()
    }

    pub(super) fn capacity_conflicts<S>(
        &self,
        group: &CoverageGroupBinding<S>,
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

    fn remove_current<S>(
        &mut self,
        group: &CoverageGroupBinding<S>,
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
        self.values[entity_index] = None;
    }

    fn occupant_order_key<S>(
        &self,
        group: &CoverageGroupBinding<S>,
        solution: &S,
        entity_index: usize,
    ) -> (bool, i64, usize) {
        (
            !self.is_required(entity_index),
            group.entity_order_key(solution, entity_index),
            entity_index,
        )
    }
}
