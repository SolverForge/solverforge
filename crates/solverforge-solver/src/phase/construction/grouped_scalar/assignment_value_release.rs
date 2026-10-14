use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_index::AssignedValueSequenceIndex;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) struct ValueRunReleaseCursor {
    value_pos: usize,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

impl ValueRunReleaseCursor {
    pub(super) fn new(
        options: ScalarAssignmentMoveOptions,
        index: &AssignedValueSequenceIndex,
    ) -> Self {
        let max_len = options
            .max_rematch_size
            .min(index.sequence_keys.len())
            .max(options.max_depth.max(2));
        Self {
            value_pos: 0,
            start_pos: 0,
            len: 2,
            max_len,
        }
    }

    pub(super) fn next<S>(
        &mut self,
        index: &AssignedValueSequenceIndex,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &mut ScalarAssignmentState,
    ) -> Option<CompoundScalarMove<S>>
    where
        S: PlanningSolution,
    {
        while self.value_pos < index.values.len() {
            while self.start_pos < index.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > index.sequence_keys.len() {
                        break;
                    }
                    let value = index.values[self.value_pos];
                    let sequence_window =
                        &index.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = self.window_release_edits(index, state, value, sequence_window);
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
                        "scalar_assignment_value_run_release",
                    ) {
                        return Some(mov);
                    }
                }
                self.start_pos += 1;
                self.len = 2;
            }
            self.value_pos += 1;
            self.start_pos = 0;
            self.len = 2;
        }
        None
    }

    fn window_release_edits(
        &self,
        index: &AssignedValueSequenceIndex,
        state: &ScalarAssignmentState,
        value: usize,
        sequence_window: &[usize],
    ) -> Vec<(usize, Option<usize>)> {
        let mut edits = Vec::new();
        for sequence_key in sequence_window {
            let Some(entities) = index.by_value_sequence.get(&(value, *sequence_key)) else {
                return Vec::new();
            };
            for entity_index in entities {
                if !state.is_required(*entity_index) {
                    edits.push((*entity_index, None));
                }
            }
        }
        edits
    }
}
