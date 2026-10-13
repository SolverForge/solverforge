use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{order_candidates, ScalarAssignmentMoveOptions};
use super::assignment_entity::assigned_entities_by_position;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

enum CycleWindowKind {
    AugmentingRematch,
    EjectionReinsert,
}

pub(super) struct CycleWindowCursor {
    kind: CycleWindowKind,
    entities: Vec<usize>,
    start: usize,
    len: usize,
    variant: usize,
    max_len: usize,
    options: ScalarAssignmentMoveOptions,
}

impl CycleWindowCursor {
    pub(super) fn augmenting<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut entities = assigned_entities_by_position(group, solution, state);
        order_candidates(&mut entities, options);
        let max_len = options.max_rematch_size.min(entities.len()).max(2);
        Self {
            kind: CycleWindowKind::AugmentingRematch,
            entities,
            start: 0,
            len: 3,
            variant: 0,
            max_len,
            options,
        }
    }

    pub(super) fn ejection<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        let mut entities = assigned_entities_by_position(group, solution, state);
        order_candidates(&mut entities, options);
        let max_len = options.max_rematch_size.min(entities.len()).max(2);
        Self {
            kind: CycleWindowKind::EjectionReinsert,
            entities,
            start: 0,
            len: 2,
            variant: 0,
            max_len,
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
            CycleWindowKind::AugmentingRematch => {
                self.next_augmenting_rematch(group, solution, state)
            }
            CycleWindowKind::EjectionReinsert => self.next_ejection(group, solution, state),
        }
    }

    fn next_augmenting_rematch<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.start < self.entities.len() {
            while self.len <= self.max_len {
                if self.start + self.len > self.entities.len() {
                    break;
                }
                while self.variant < 2 {
                    let direction = if self.variant == 0 { 1 } else { self.len - 1 };
                    self.variant += 1;
                    let window = &self.entities[self.start..self.start + self.len];
                    let mut edits = Vec::with_capacity(self.len);
                    for (offset, &entity_index) in window.iter().enumerate() {
                        let source_entity = window[(offset + direction) % self.len];
                        let Some(value) = state.current_value(source_entity) else {
                            edits.clear();
                            break;
                        };
                        if state.current_value(entity_index) != Some(value) {
                            edits.push((entity_index, Some(value)));
                        }
                    }
                    if edits.len() < 2
                        || !state.assignment_feasible_after_edits(group, solution, &edits)
                    {
                        continue;
                    }
                    let scalar_edits = edits
                        .iter()
                        .map(|(entity_index, value)| group.edit(*entity_index, *value))
                        .collect::<Vec<_>>();
                    if let Some(mov) = move_from_edits(
                        group,
                        solution,
                        &scalar_edits,
                        "scalar_assignment_augmenting_rematch",
                    ) {
                        return Some(mov);
                    }
                }
                self.len += 1;
                self.variant = 0;
            }
            self.start += 1;
            self.len = 3;
            self.variant = 0;
        }
        None
    }

    fn next_ejection<S>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        let attempt_count = self.options.max_depth.max(1);
        while self.start < self.entities.len() {
            while self.len <= self.max_len {
                if self.start + self.len > self.entities.len() {
                    break;
                }
                let window = &self.entities[self.start..self.start + self.len];
                if !window
                    .iter()
                    .any(|entity_index| state.is_required(*entity_index))
                {
                    self.len += 1;
                    self.variant = 0;
                    continue;
                }
                while self.variant < attempt_count {
                    let attempt = self.variant;
                    self.variant += 1;
                    let Some(edits) = ejection_reinsert_edits(
                        group,
                        solution,
                        state,
                        window,
                        attempt,
                        self.options,
                    ) else {
                        continue;
                    };
                    if edits.len() < 2
                        || !state.assignment_feasible_after_edits(group, solution, &edits)
                    {
                        continue;
                    }
                    let scalar_edits = edits
                        .iter()
                        .map(|(entity_index, value)| group.edit(*entity_index, *value))
                        .collect::<Vec<_>>();
                    if let Some(mov) = move_from_edits(
                        group,
                        solution,
                        &scalar_edits,
                        "scalar_assignment_ejection_reinsert",
                    ) {
                        return Some(mov);
                    }
                }
                self.len += 1;
                self.variant = 0;
            }
            self.start += 1;
            self.len = 2;
            self.variant = 0;
        }
        None
    }
}

fn ejection_reinsert_edits<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    window: &[usize],
    attempt: usize,
    options: ScalarAssignmentMoveOptions,
) -> Option<Vec<(usize, Option<usize>)>>
where
    S: PlanningSolution,
{
    let mut edits = window
        .iter()
        .map(|entity| (*entity, None))
        .collect::<Vec<_>>();
    for offset in 0..window.len() {
        let position = (attempt % window.len() + offset) % window.len();
        let entity = window[position];
        let values = group.candidate_values(solution, entity, options.value_candidate_limit);
        let mut found = false;
        for value_offset in 0..values.len() {
            let value = values[(attempt % values.len() + value_offset) % values.len()];
            edits[position].1 = Some(value);
            if state.reinsert_value_allowed(group, solution, &edits, entity, value) {
                found = true;
                break;
            }
        }
        if !found {
            edits[position].1 = None;
            if state.is_required(entity) {
                return None;
            }
        }
    }
    edits.retain(|(entity, next)| state.current_value(*entity) != *next);
    Some(edits)
}
