use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_coordinates::{ValuePairCoordinates, ValuePairKind};
use super::assignment_value_index::AssignedValueSequenceIndex;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use solverforge_core::fast_hash::{FastMap, FastSet};

pub(super) struct ValueWindowCursor {
    pairs: ValuePairCoordinates,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

pub(super) struct ValueLongWindowCursor {
    pairs: ValuePairCoordinates,
    length_pos: usize,
    start_pos: usize,
    lengths: Vec<usize>,
}

pub(super) struct ValueBlockReassignmentCursor {
    pairs: ValuePairCoordinates,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

impl ValueLongWindowCursor {
    pub(super) fn new(
        options: ScalarAssignmentMoveOptions,
        index: &AssignedValueSequenceIndex,
    ) -> Self {
        let pairs = ValuePairCoordinates::new(&index.values, ValuePairKind::Unordered, options);

        let base_len = options
            .max_rematch_size
            .min(index.sequence_keys.len())
            .max(options.max_depth.max(2));
        let lengths = vec![base_len];
        Self {
            pairs,
            length_pos: 0,
            start_pos: 0,
            lengths,
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
        while let Some(pair) = self.pairs.current(&index.values) {
            while self.length_pos < self.lengths.len() {
                let len = self.lengths[self.length_pos];
                while self.start_pos + len <= index.sequence_keys.len() {
                    let (left_value, right_value) = pair;
                    let sequence_window =
                        &index.sequence_keys[self.start_pos..self.start_pos + len];
                    self.start_pos += 1;
                    let edits = value_window_edits(
                        &index.by_value_sequence,
                        group,
                        solution,
                        state,
                        left_value,
                        right_value,
                        sequence_window,
                    );
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
                        "scalar_assignment_value_long_window_swap",
                    ) {
                        return Some(mov);
                    }
                }
                self.length_pos += 1;
                self.start_pos = 0;
            }
            self.pairs.advance();
            self.length_pos = 0;
            self.start_pos = 0;
        }
        None
    }
}

impl ValueWindowCursor {
    pub(super) fn new(
        options: ScalarAssignmentMoveOptions,
        index: &AssignedValueSequenceIndex,
    ) -> Self {
        let pairs = ValuePairCoordinates::new(&index.values, ValuePairKind::Unordered, options);

        let max_len = options
            .max_depth
            .max(3)
            .min(index.sequence_keys.len())
            .max(2);
        Self {
            pairs,
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
        while let Some(pair) = self.pairs.current(&index.values) {
            while self.start_pos < index.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > index.sequence_keys.len() {
                        break;
                    }
                    let (left_value, right_value) = pair;
                    let sequence_window =
                        &index.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = value_window_edits(
                        &index.by_value_sequence,
                        group,
                        solution,
                        state,
                        left_value,
                        right_value,
                        sequence_window,
                    );
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
                        "scalar_assignment_value_window_swap",
                    ) {
                        return Some(mov);
                    }
                }
                self.start_pos += 1;
                self.len = 2;
            }
            self.pairs.advance();
            self.start_pos = 0;
            self.len = 2;
        }
        None
    }
}

impl ValueBlockReassignmentCursor {
    pub(super) fn new(
        options: ScalarAssignmentMoveOptions,
        index: &AssignedValueSequenceIndex,
    ) -> Self {
        let pairs = ValuePairCoordinates::new(&index.values, ValuePairKind::Directed, options);

        let max_len = options
            .max_depth
            .max(2)
            .min(index.sequence_keys.len())
            .max(2);
        Self {
            pairs,
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
        while let Some(pair) = self.pairs.current(&index.values) {
            while self.start_pos < index.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > index.sequence_keys.len() {
                        break;
                    }
                    let (source_value, target_value) = pair;
                    let sequence_window =
                        &index.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = self.window_edits(
                        index,
                        group,
                        solution,
                        state,
                        source_value,
                        target_value,
                        sequence_window,
                    );
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
                        "scalar_assignment_value_block_reassignment",
                    ) {
                        return Some(mov);
                    }
                }
                self.start_pos += 1;
                self.len = 2;
            }
            self.pairs.advance();
            self.start_pos = 0;
            self.len = 2;
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn window_edits<S>(
        &self,
        index: &AssignedValueSequenceIndex,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        source_value: usize,
        target_value: usize,
        sequence_window: &[usize],
    ) -> Vec<(usize, Option<usize>)>
    where
        S: PlanningSolution,
    {
        let mut edits = Vec::new();
        for sequence_key in sequence_window {
            if let Some(source_entities) =
                index.by_value_sequence.get(&(source_value, *sequence_key))
            {
                for entity_index in source_entities {
                    if state.current_value(*entity_index) != Some(target_value)
                        && group.value_is_legal(solution, *entity_index, Some(target_value))
                    {
                        edits.push((*entity_index, Some(target_value)));
                    }
                }
            }
        }
        edits
    }
}

#[allow(clippy::too_many_arguments)]
fn value_window_edits<S>(
    by_value_sequence: &FastMap<(usize, usize), Vec<usize>>,
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &ScalarAssignmentState,
    left_value: usize,
    right_value: usize,
    sequence_window: &[usize],
) -> Vec<(usize, Option<usize>)>
where
    S: PlanningSolution,
{
    let mut edits = Vec::new();
    let mut touched = FastSet::default();
    for sequence_key in sequence_window {
        if let Some(left_entities) = by_value_sequence.get(&(left_value, *sequence_key)) {
            for entity_index in left_entities {
                if touched.insert(*entity_index)
                    && state.current_value(*entity_index) != Some(right_value)
                    && group.value_is_legal(solution, *entity_index, Some(right_value))
                {
                    edits.push((*entity_index, Some(right_value)));
                }
            }
        }
        if let Some(right_entities) = by_value_sequence.get(&(right_value, *sequence_key)) {
            for entity_index in right_entities {
                if touched.insert(*entity_index)
                    && state.current_value(*entity_index) != Some(left_value)
                    && group.value_is_legal(solution, *entity_index, Some(left_value))
                {
                    edits.push((*entity_index, Some(left_value)));
                }
            }
        }
    }
    edits
}
