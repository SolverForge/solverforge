use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_index::assigned_value_sequence_index;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) struct ValueWindowCycleCursor {
    triples: Vec<(usize, usize, usize)>,
    sequence_keys: Vec<usize>,
    by_value_sequence: HashMap<(usize, usize), Vec<usize>>,
    triple_pos: usize,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

impl ValueWindowCycleCursor {
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
        let mut triples = Vec::new();
        for first_pos in 0..index.values.len() {
            for second_pos in first_pos + 1..index.values.len() {
                for third_pos in second_pos + 1..index.values.len() {
                    let first = index.values[first_pos];
                    let second = index.values[second_pos];
                    let third = index.values[third_pos];
                    triples.push((first, second, third));
                    triples.push((first, third, second));
                }
            }
        }
        if !triples.is_empty() {
            let triple_count = triples.len();
            triples.rotate_left(options.entity_offset % triple_count);
        }

        let max_len = options
            .max_depth
            .max(2)
            .min(index.sequence_keys.len())
            .max(2);
        Self {
            triples,
            sequence_keys: index.sequence_keys,
            by_value_sequence: index.by_value_sequence,
            triple_pos: 0,
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
        while self.triple_pos < self.triples.len() {
            while self.start_pos < self.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > self.sequence_keys.len() {
                        break;
                    }
                    let (first, second, third) = self.triples[self.triple_pos];
                    let sequence_window =
                        &self.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = self.window_edits(
                        group,
                        solution,
                        state,
                        first,
                        second,
                        third,
                        sequence_window,
                    );
                    if edits.len() < 3
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
                        "scalar_assignment_value_window_cycle",
                    ) {
                        return Some(mov);
                    }
                }
                self.start_pos += 1;
                self.len = 2;
            }
            self.triple_pos += 1;
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
        first: usize,
        second: usize,
        third: usize,
        sequence_window: &[usize],
    ) -> Vec<(usize, Option<usize>)>
    where
        S: PlanningSolution,
    {
        let mut edits = Vec::new();
        let mut touched = HashSet::new();
        for sequence_key in sequence_window {
            push_cycle_edits(
                &self.by_value_sequence,
                &mut touched,
                &mut edits,
                group,
                solution,
                state,
                first,
                second,
                *sequence_key,
            );
            push_cycle_edits(
                &self.by_value_sequence,
                &mut touched,
                &mut edits,
                group,
                solution,
                state,
                second,
                third,
                *sequence_key,
            );
            push_cycle_edits(
                &self.by_value_sequence,
                &mut touched,
                &mut edits,
                group,
                solution,
                state,
                third,
                first,
                *sequence_key,
            );
        }
        edits
    }
}

fn push_cycle_edits<S>(
    by_value_sequence: &HashMap<(usize, usize), Vec<usize>>,
    touched: &mut HashSet<usize>,
    edits: &mut Vec<(usize, Option<usize>)>,
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &ScalarAssignmentState,
    from_value: usize,
    to_value: usize,
    sequence_key: usize,
) where
    S: PlanningSolution,
{
    let Some(entities) = by_value_sequence.get(&(from_value, sequence_key)) else {
        return;
    };
    for entity_index in entities {
        if touched.insert(*entity_index)
            && state.current_value(*entity_index) != Some(to_value)
            && group.value_is_legal(solution, *entity_index, Some(to_value))
        {
            edits.push((*entity_index, Some(to_value)));
        }
    }
}
