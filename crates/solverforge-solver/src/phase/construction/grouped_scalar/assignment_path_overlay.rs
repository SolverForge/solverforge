use solverforge_core::fast_hash::FastMap;

use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;

#[derive(Default)]
pub(super) struct AssignmentPathOverlay {
    changed: FastMap<usize, Option<usize>>,
    journal: Vec<(usize, Option<Option<usize>>)>,
    left_members: Vec<usize>,
    right_members: Vec<usize>,
}

impl AssignmentPathOverlay {
    pub(super) fn clear(&mut self) {
        self.changed.clear();
        self.journal.clear();
        self.left_members.clear();
        self.right_members.clear();
    }

    pub(super) fn checkpoint(&self) -> usize {
        self.journal.len()
    }

    pub(super) fn current_value(
        &self,
        state: &ScalarAssignmentState,
        entity: usize,
    ) -> Option<usize> {
        self.changed
            .get(&entity)
            .copied()
            .unwrap_or_else(|| state.current_value(entity))
    }

    pub(super) fn set_value(
        &mut self,
        state: &ScalarAssignmentState,
        entity: usize,
        value: Option<usize>,
    ) {
        if self.current_value(state, entity) == value {
            return;
        }
        let previous = self.changed.insert(entity, value);
        self.journal.push((entity, previous));
    }

    pub(super) fn rollback(&mut self, checkpoint: usize) {
        while self.journal.len() > checkpoint {
            let (entity, previous) = self.journal.pop().expect("journal length was checked");
            match previous {
                Some(value) => {
                    self.changed.insert(entity, value);
                }
                None => {
                    self.changed.remove(&entity);
                }
            }
        }
    }

    pub(super) fn blockers<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
        blockers: &mut Vec<usize>,
    ) -> Option<bool> {
        blockers.clear();
        if let Some(key) = group.capacity_key(solution, entity, value) {
            for occupant in state.capacity_occupants(key) {
                if *occupant != entity
                    && self.current_value(state, *occupant).is_some_and(|current| {
                        group.capacity_key(solution, *occupant, current) == Some(key)
                    })
                {
                    blockers.push(*occupant);
                }
            }
            for (changed, next) in &self.changed {
                if *changed == entity
                    || state.capacity_occupants(key).contains(changed)
                    || !next.is_some_and(|next| {
                        group.capacity_key(solution, *changed, next) == Some(key)
                    })
                {
                    continue;
                }
                blockers.push(*changed);
            }
        }

        let verdict = self.push_rule_blockers(state, group, solution, entity, value, blockers);
        blockers.sort_unstable();
        blockers.dedup();
        verdict
    }

    pub(super) fn assignment_allowed<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
    ) -> bool {
        group.value_is_legal(solution, entity, Some(value))
            && self.rule_allows(state, group, solution, entity, value)
    }

    fn push_rule_blockers<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
        blockers: &mut Vec<usize>,
    ) -> Option<bool> {
        if !group.has_assignment_rule() {
            return None;
        }
        if self.rule_allows(state, group, solution, entity, value) {
            return Some(true);
        }
        let Some(sequence) = group.sequence_key(solution, entity, value) else {
            return Some(false);
        };
        for adjacent in [sequence.checked_sub(1), sequence.checked_add(1)]
            .into_iter()
            .flatten()
        {
            self.collect_members(state, group, solution, value, adjacent, true);
            blockers.extend(
                self.left_members
                    .iter()
                    .copied()
                    .filter(|blocker| *blocker != entity),
            );
        }
        Some(false)
    }

    fn rule_allows<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity: usize,
        value: usize,
    ) -> bool {
        if !group.has_assignment_rule() {
            return true;
        }
        let Some(sequence) = group.sequence_key(solution, entity, value) else {
            return true;
        };
        if let Some(previous) = sequence.checked_sub(1) {
            self.collect_members(state, group, solution, value, previous, true);
            if self.left_members.iter().copied().any(|left| {
                left != entity
                    && !group.assignment_edge_allowed(solution, left, value, entity, value)
            }) {
                return false;
            }
        }
        if let Some(next) = sequence.checked_add(1) {
            self.collect_members(state, group, solution, value, next, false);
            if self.right_members.iter().copied().any(|right| {
                right != entity
                    && !group.assignment_edge_allowed(solution, entity, value, right, value)
            }) {
                return false;
            }
        }
        true
    }

    fn collect_members<S>(
        &mut self,
        state: &ScalarAssignmentState,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        value: usize,
        sequence: usize,
        left: bool,
    ) {
        let changed = &self.changed;
        let members = if left {
            &mut self.left_members
        } else {
            &mut self.right_members
        };
        members.clear();
        for entity in state.sequence_members(value, sequence) {
            if changed
                .get(entity)
                .copied()
                .unwrap_or_else(|| state.current_value(*entity))
                == Some(value)
            {
                members.push(*entity);
            }
        }
        for (entity, next) in &self.changed {
            if state.sequence_members(value, sequence).contains(entity)
                || *next != Some(value)
                || group.sequence_key(solution, *entity, value) != Some(sequence)
            {
                continue;
            }
            members.push(*entity);
        }
    }
}
