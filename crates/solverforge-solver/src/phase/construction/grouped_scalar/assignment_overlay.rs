use super::{pair_was_checked, ScalarAssignmentBinding, ScalarAssignmentState};
use crate::planning::ScalarEdit;
use solverforge_core::fast_hash::{FastMap, FastSet};

#[derive(Default)]
pub(super) struct OverlayScratch {
    targets: FastSet<usize>,
    changed: FastMap<usize, Option<usize>>,
    capacity_deltas: FastMap<usize, isize>,
    arrivals: FastMap<(usize, usize), Vec<usize>>,
    rows: Vec<Vec<usize>>,
    edited: Vec<(usize, Option<usize>, Option<usize>)>,
    checked: Vec<(usize, usize)>,
}

impl ScalarAssignmentState {
    pub(in super::super) fn assignment_feasible_after_edits<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: &[(usize, Option<usize>)],
    ) -> bool {
        self.final_edits_feasible(group, solution, edits.iter().copied())
    }

    pub(in super::super) fn scalar_edits_feasible<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: &[ScalarEdit<S>],
    ) -> bool {
        self.final_edits_feasible(
            group,
            solution,
            edits
                .iter()
                .map(|edit| (edit.entity_index(), edit.to_value())),
        )
    }

    fn final_edits_feasible<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: impl IntoIterator<Item = (usize, Option<usize>)>,
    ) -> bool {
        let mut overlay = std::mem::take(&mut self.overlay);
        let feasible = overlay.prepare(self, group, solution, edits, true)
            && overlay.capacity_feasible(self)
            && overlay.rules_feasible(self, group, solution);
        self.overlay = overlay;
        feasible
    }
}

impl ScalarAssignmentState {
    pub(in super::super) fn reinsert_value_allowed<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: &[(usize, Option<usize>)],
        entity: usize,
        value: usize,
    ) -> bool {
        let mut overlay = std::mem::take(&mut self.overlay);
        let allowed = overlay.prepare(self, group, solution, edits.iter().copied(), false)
            && overlay.reinsert_allowed(self, group, solution, entity, value);
        self.overlay = overlay;
        allowed
    }
}

impl OverlayScratch {
    fn prepare<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        edits: impl IntoIterator<Item = (usize, Option<usize>)>,
        final_assignment: bool,
    ) -> bool {
        self.targets.clear();
        self.changed.clear();
        self.capacity_deltas.clear();
        self.edited.clear();
        self.checked.clear();
        for (_, mut row) in self.arrivals.drain() {
            row.clear();
            self.rows.push(row);
        }
        for (entity, next) in edits {
            if entity >= state.values.len()
                || !self.targets.insert(entity)
                || (final_assignment && next.is_none() && state.is_required(entity))
                || !group.value_is_legal(solution, entity, next)
            {
                return false;
            }
            let previous = state.current_value(entity);
            self.edited.push((entity, previous, next));
            if let Some(key) = state.capacity_keys[entity] {
                *self.capacity_deltas.entry(key).or_default() -= 1;
            }
            if let Some(key) = next.and_then(|v| group.capacity_key(solution, entity, v)) {
                *self.capacity_deltas.entry(key).or_default() += 1;
            }
            if previous == next {
                continue;
            }
            self.changed.insert(entity, next);
            if let Some(value) = next {
                if let Some(sequence) = group.sequence_key(solution, entity, value) {
                    self.arrivals
                        .entry((value, sequence))
                        .or_insert_with(|| self.rows.pop().unwrap_or_default())
                        .push(entity);
                }
            }
        }
        true
    }

    fn reinsert_allowed<S>(
        &self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
    ) -> bool {
        if let Some(key) = group.capacity_key(solution, entity, value) {
            let count = state.occupancy.get(&key).map_or(0, Vec::len);
            let delta = self.capacity_deltas.get(&key).copied().unwrap_or(0);
            if !count.checked_add_signed(delta).is_some_and(|n| n <= 1) {
                return false;
            }
        }
        if !group.has_assignment_rule() {
            return true;
        }
        let Some(sequence) = group.sequence_key(solution, entity, value) else {
            return true;
        };
        if let Some(previous) = sequence.checked_sub(1) {
            for left in self.members(state, value, previous) {
                if left != entity
                    && !group.assignment_edge_allowed(solution, left, value, entity, value)
                {
                    return false;
                }
            }
        }
        if let Some(next) = sequence.checked_add(1) {
            for right in self.members(state, value, next) {
                if right != entity
                    && !group.assignment_edge_allowed(solution, entity, value, right, value)
                {
                    return false;
                }
            }
        }
        true
    }

    fn capacity_feasible(&self, state: &ScalarAssignmentState) -> bool {
        self.capacity_deltas.iter().all(|(key, delta)| {
            let count = state.occupancy.get(key).map_or(0, Vec::len);
            count.checked_add_signed(*delta).is_some_and(|n| n <= 1)
        })
    }

    fn members<'a>(
        &'a self,
        state: &'a ScalarAssignmentState,
        value: usize,
        sequence: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        state
            .assigned_by_sequence
            .get(&(value, sequence))
            .into_iter()
            .flatten()
            .copied()
            .filter(|entity| !self.changed.contains_key(entity))
            .chain(
                self.arrivals
                    .get(&(value, sequence))
                    .into_iter()
                    .flatten()
                    .copied(),
            )
    }

    fn rules_feasible<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
    ) -> bool {
        if !group.has_assignment_rule() {
            return true;
        }
        let mut checked = std::mem::take(&mut self.checked);
        let mut feasible = true;
        'edits: for (entity, previous, next) in &self.edited {
            for (membership, value) in [*previous, *next].into_iter().enumerate() {
                let Some(value) = value else {
                    continue;
                };
                let sequence = if membership == 0 {
                    state.recorded_sequence_key(*entity)
                } else {
                    group.sequence_key(solution, *entity, value)
                };
                let Some(sequence) = sequence else {
                    continue;
                };
                let edges = [
                    sequence.checked_sub(1).map(|left| (left, sequence)),
                    sequence.checked_add(1).map(|right| (sequence, right)),
                ];
                for (left_sequence, right_sequence) in edges.into_iter().flatten() {
                    for left in self.members(state, value, left_sequence) {
                        for right in self.members(state, value, right_sequence) {
                            if left == right || pair_was_checked(&checked, left, right) {
                                continue;
                            }
                            checked.push((left, right));
                            if !group.assignment_edge_allowed(solution, left, value, right, value) {
                                feasible = false;
                                break 'edits;
                            }
                        }
                    }
                }
            }
        }
        self.checked = checked;
        feasible
    }
}
