use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_index::assigned_value_sequence_index;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) struct ValueWindowCursor {
    pairs: Vec<(usize, usize)>,
    sequence_keys: Vec<usize>,
    by_value_sequence: HashMap<(usize, usize), Vec<usize>>,
    pair_pos: usize,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

pub(super) struct ValueLongWindowCursor {
    pairs: Vec<(usize, usize)>,
    sequence_keys: Vec<usize>,
    by_value_sequence: HashMap<(usize, usize), Vec<usize>>,
    pair_pos: usize,
    length_pos: usize,
    start_pos: usize,
    lengths: Vec<usize>,
}

pub(super) struct ValueBlockReassignmentCursor {
    pairs: Vec<(usize, usize)>,
    sequence_keys: Vec<usize>,
    by_value_sequence: HashMap<(usize, usize), Vec<usize>>,
    pair_pos: usize,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

impl ValueLongWindowCursor {
    pub(super) fn new<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self
    where
        S: PlanningSolution,
    {
        let index = assigned_value_sequence_index(group, solution, state, options);

        let mut pairs = Vec::new();
        for left_pos in 0..index.values.len() {
            for right_pos in left_pos + 1..index.values.len() {
                pairs.push((index.values[left_pos], index.values[right_pos]));
            }
        }
        if !pairs.is_empty() {
            let pair_count = pairs.len();
            pairs.rotate_left(options.entity_offset % pair_count);
        }

        let base_len = options
            .max_rematch_size
            .min(index.sequence_keys.len())
            .max(options.max_depth.max(2));
        let lengths = vec![base_len];
        Self {
            pairs,
            sequence_keys: index.sequence_keys,
            by_value_sequence: index.by_value_sequence,
            pair_pos: 0,
            length_pos: 0,
            start_pos: 0,
            lengths,
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
        while self.pair_pos < self.pairs.len() {
            while self.length_pos < self.lengths.len() {
                let len = self.lengths[self.length_pos];
                while self.start_pos + len <= self.sequence_keys.len() {
                    let (left_value, right_value) = self.pairs[self.pair_pos];
                    let sequence_window = &self.sequence_keys[self.start_pos..self.start_pos + len];
                    self.start_pos += 1;
                    let edits = value_window_edits(
                        &self.by_value_sequence,
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
            self.pair_pos += 1;
            self.length_pos = 0;
            self.start_pos = 0;
        }
        None
    }
}

impl ValueWindowCursor {
    pub(super) fn new<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self
    where
        S: PlanningSolution,
    {
        let index = assigned_value_sequence_index(group, solution, state, options);

        let mut pairs = Vec::new();
        for left_pos in 0..index.values.len() {
            for right_pos in left_pos + 1..index.values.len() {
                pairs.push((index.values[left_pos], index.values[right_pos]));
            }
        }
        if !pairs.is_empty() {
            let pair_count = pairs.len();
            pairs.rotate_left(options.entity_offset % pair_count);
        }

        let max_len = options
            .max_depth
            .max(3)
            .min(index.sequence_keys.len())
            .max(2);
        Self {
            pairs,
            sequence_keys: index.sequence_keys,
            by_value_sequence: index.by_value_sequence,
            pair_pos: 0,
            start_pos: 0,
            len: 2,
            max_len,
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
        while self.pair_pos < self.pairs.len() {
            while self.start_pos < self.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > self.sequence_keys.len() {
                        break;
                    }
                    let (left_value, right_value) = self.pairs[self.pair_pos];
                    let sequence_window =
                        &self.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = value_window_edits(
                        &self.by_value_sequence,
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
            self.pair_pos += 1;
            self.start_pos = 0;
            self.len = 2;
        }
        None
    }
}

impl ValueBlockReassignmentCursor {
    pub(super) fn new<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self
    where
        S: PlanningSolution,
    {
        let index = assigned_value_sequence_index(group, solution, state, options);
        let mut pairs = Vec::new();
        for source in &index.values {
            for target in &index.values {
                if source != target {
                    pairs.push((*source, *target));
                }
            }
        }
        if !pairs.is_empty() {
            let pair_count = pairs.len();
            pairs.rotate_left(options.entity_offset % pair_count);
        }

        let max_len = options
            .max_depth
            .max(2)
            .min(index.sequence_keys.len())
            .max(2);
        Self {
            pairs,
            sequence_keys: index.sequence_keys,
            by_value_sequence: index.by_value_sequence,
            pair_pos: 0,
            start_pos: 0,
            len: 2,
            max_len,
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
        while self.pair_pos < self.pairs.len() {
            while self.start_pos < self.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > self.sequence_keys.len() {
                        break;
                    }
                    let (source_value, target_value) = self.pairs[self.pair_pos];
                    let sequence_window =
                        &self.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = self.window_edits(
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
            self.pair_pos += 1;
            self.start_pos = 0;
            self.len = 2;
        }
        None
    }

    fn window_edits<S>(
        &self,
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
                self.by_value_sequence.get(&(source_value, *sequence_key))
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

fn value_window_edits<S>(
    by_value_sequence: &HashMap<(usize, usize), Vec<usize>>,
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
    let mut touched = HashSet::new();
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
