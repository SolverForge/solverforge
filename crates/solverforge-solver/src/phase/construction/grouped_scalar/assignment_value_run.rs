use std::collections::HashMap;

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_index::assigned_value_sequence_index;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) struct ValueRunGapSwapCursor {
    pairs: Vec<(usize, usize)>,
    sequence_keys: Vec<usize>,
    by_value_sequence: HashMap<(usize, usize), Vec<usize>>,
    pair_pos: usize,
    fill_pos: usize,
    release_pos: usize,
    fill_sequences: Vec<usize>,
    release_sequences: Vec<usize>,
}

impl ValueRunGapSwapCursor {
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
        for target in &index.values {
            for source in &index.values {
                if target != source {
                    pairs.push((*target, *source));
                }
            }
        }
        if !pairs.is_empty() {
            let pair_count = pairs.len();
            pairs.rotate_left(options.entity_offset % pair_count);
        }

        Self {
            pairs,
            sequence_keys: index.sequence_keys,
            by_value_sequence: index.by_value_sequence,
            pair_pos: 0,
            fill_pos: 0,
            release_pos: 0,
            fill_sequences: Vec::new(),
            release_sequences: Vec::new(),
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
            self.ensure_pair_loaded();
            while self.fill_pos < self.fill_sequences.len() {
                while self.release_pos < self.release_sequences.len() {
                    let (target_value, source_value) = self.pairs[self.pair_pos];
                    let fill_sequence = self.fill_sequences[self.fill_pos];
                    let release_sequence = self.release_sequences[self.release_pos];
                    self.release_pos += 1;
                    let edits = self.swap_edits(
                        group,
                        solution,
                        state,
                        target_value,
                        source_value,
                        fill_sequence,
                        release_sequence,
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
                        "scalar_assignment_value_run_gap_swap",
                    ) {
                        return Some(mov);
                    }
                }
                self.fill_pos += 1;
                self.release_pos = 0;
            }
            self.advance_pair();
        }
        None
    }

    fn ensure_pair_loaded(&mut self) {
        if !self.fill_sequences.is_empty() || !self.release_sequences.is_empty() {
            return;
        }
        let (target_value, source_value) = self.pairs[self.pair_pos];
        self.fill_sequences = self
            .sequence_keys
            .iter()
            .copied()
            .filter(|sequence| {
                self.has_value(source_value, *sequence)
                    && self.is_single_gap(target_value, *sequence)
            })
            .collect();
        self.release_sequences = self
            .sequence_keys
            .iter()
            .copied()
            .filter(|sequence| {
                self.has_value(target_value, *sequence) && !self.has_value(source_value, *sequence)
            })
            .collect();
        let by_value_sequence = &self.by_value_sequence;
        self.release_sequences.sort_by_key(|sequence| {
            (
                !is_single_island(by_value_sequence, target_value, *sequence),
                *sequence,
            )
        });
    }

    fn advance_pair(&mut self) {
        self.pair_pos += 1;
        self.fill_pos = 0;
        self.release_pos = 0;
        self.fill_sequences.clear();
        self.release_sequences.clear();
    }

    fn swap_edits<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        target_value: usize,
        source_value: usize,
        fill_sequence: usize,
        release_sequence: usize,
    ) -> Vec<(usize, Option<usize>)>
    where
        S: PlanningSolution,
    {
        let mut edits = Vec::new();
        self.push_reassign_edits(
            group,
            solution,
            state,
            source_value,
            target_value,
            fill_sequence,
            &mut edits,
        );
        self.push_reassign_edits(
            group,
            solution,
            state,
            target_value,
            source_value,
            release_sequence,
            &mut edits,
        );
        edits
    }

    fn push_reassign_edits<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        from_value: usize,
        to_value: usize,
        sequence: usize,
        edits: &mut Vec<(usize, Option<usize>)>,
    ) where
        S: PlanningSolution,
    {
        let Some(entities) = self.by_value_sequence.get(&(from_value, sequence)) else {
            return;
        };
        for entity_index in entities {
            if state.current_value(*entity_index) != Some(to_value)
                && group.value_is_legal(solution, *entity_index, Some(to_value))
            {
                edits.push((*entity_index, Some(to_value)));
            }
        }
    }

    fn is_single_gap(&self, value: usize, sequence: usize) -> bool {
        !self.has_value(value, sequence)
            && sequence
                .checked_sub(1)
                .is_some_and(|previous| self.has_value(value, previous))
            && sequence
                .checked_add(1)
                .is_some_and(|next| self.has_value(value, next))
    }

    fn has_value(&self, value: usize, sequence: usize) -> bool {
        self.by_value_sequence
            .get(&(value, sequence))
            .is_some_and(|entities| !entities.is_empty())
    }
}

fn is_single_island(
    by_value_sequence: &HashMap<(usize, usize), Vec<usize>>,
    value: usize,
    sequence: usize,
) -> bool {
    has_value(by_value_sequence, value, sequence)
        && sequence
            .checked_sub(1)
            .is_none_or(|previous| !has_value(by_value_sequence, value, previous))
        && sequence
            .checked_add(1)
            .is_none_or(|next| !has_value(by_value_sequence, value, next))
}

fn has_value(
    by_value_sequence: &HashMap<(usize, usize), Vec<usize>>,
    value: usize,
    sequence: usize,
) -> bool {
    by_value_sequence
        .get(&(value, sequence))
        .is_some_and(|entities| !entities.is_empty())
}
