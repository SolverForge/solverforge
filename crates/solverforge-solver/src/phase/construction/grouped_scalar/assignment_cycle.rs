use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{rotate_entity_order, ScalarAssignmentMoveOptions};
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
        rotate_entity_order(&mut entities, options.entity_offset);
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
        rotate_entity_order(&mut entities, options.entity_offset);
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
                    let Some(edits) =
                        ejection_reinsert_edits(group, solution, window, attempt, self.options)
                    else {
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
    window: &[usize],
    attempt: usize,
    options: ScalarAssignmentMoveOptions,
) -> Option<Vec<(usize, Option<usize>)>>
where
    S: PlanningSolution,
{
    let mut state = ScalarAssignmentState::new(group, solution);
    let mut changes = Vec::with_capacity(window.len() * 2);
    for entity_index in window {
        state.set_value_recording(group, solution, *entity_index, None, &mut changes);
    }

    let mut order = window.to_vec();
    rotate_entity_order(&mut order, attempt);
    for entity_index in order {
        let values = group.candidate_values(solution, entity_index, options.value_candidate_limit);
        let Some(value) =
            first_reinsert_value(group, solution, &mut state, entity_index, &values, attempt)
        else {
            if state.is_required(entity_index) {
                return None;
            }
            continue;
        };
        state.set_value_recording(group, solution, entity_index, Some(value), &mut changes);
    }

    let edits = window
        .iter()
        .filter_map(|entity_index| {
            let next = state.current_value(*entity_index);
            (next != group.current_value(solution, *entity_index)).then_some((*entity_index, next))
        })
        .collect::<Vec<_>>();
    Some(edits)
}

fn first_reinsert_value<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    entity_index: usize,
    values: &[usize],
    attempt: usize,
) -> Option<usize>
where
    S: PlanningSolution,
{
    if values.is_empty() {
        return None;
    }
    for offset in 0..values.len() {
        let value = values[(attempt + offset) % values.len()];
        if state
            .blockers(group, solution, entity_index, value)
            .is_empty()
            && state.assignment_allowed(group, solution, entity_index, value)
        {
            return Some(value);
        }
    }
    None
}
