use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_coordinates::{ValuePairCoordinates, ValuePairKind};
use super::assignment_value_index::AssignedValueSequenceIndex;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use solverforge_core::fast_hash::FastMap;

pub(super) struct ValueRunGapSwapCursor {
    pairs: ValuePairCoordinates,
    fill_pos: usize,
    release_pos: usize,
    fill_sequences: Vec<usize>,
    release_sequences: Vec<usize>,
}

#[derive(Clone, Copy)]
struct RunGapWindow {
    target_value: usize,
    source_value: usize,
    fill_sequence: usize,
    release_sequence: usize,
}

#[derive(Clone, Copy)]
struct ValueStep {
    from_value: usize,
    to_value: usize,
    sequence: usize,
}

impl ValueRunGapSwapCursor {
    pub(super) fn new(
        options: ScalarAssignmentMoveOptions,
        index: &AssignedValueSequenceIndex,
    ) -> Self {
        let pairs = ValuePairCoordinates::new(&index.values, ValuePairKind::Directed, options);

        Self {
            pairs,
            fill_pos: 0,
            release_pos: 0,
            fill_sequences: Vec::new(),
            release_sequences: Vec::new(),
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
        while let Some((target_value, source_value)) = self.pairs.current(&index.values) {
            self.ensure_pair_loaded(index);
            while self.fill_pos < self.fill_sequences.len() {
                while self.release_pos < self.release_sequences.len() {
                    let fill_sequence = self.fill_sequences[self.fill_pos];
                    let release_sequence = self.release_sequences[self.release_pos];
                    self.release_pos += 1;
                    let edits = self.swap_edits(
                        index,
                        group,
                        solution,
                        state,
                        RunGapWindow {
                            target_value,
                            source_value,
                            fill_sequence,
                            release_sequence,
                        },
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

    fn ensure_pair_loaded(&mut self, index: &AssignedValueSequenceIndex) {
        if !self.fill_sequences.is_empty() || !self.release_sequences.is_empty() {
            return;
        }
        let Some((target_value, source_value)) = self.pairs.current(&index.values) else {
            return;
        };
        self.fill_sequences = index
            .sequence_keys
            .iter()
            .copied()
            .filter(|sequence| {
                index.has_value(source_value, *sequence)
                    && is_single_gap(index, target_value, *sequence)
            })
            .collect();
        self.release_sequences = index
            .sequence_keys
            .iter()
            .copied()
            .filter(|sequence| {
                index.has_value(target_value, *sequence)
                    && !index.has_value(source_value, *sequence)
            })
            .collect();
        self.release_sequences.sort_by_key(|sequence| {
            (
                !is_single_island(&index.by_value_sequence, target_value, *sequence),
                *sequence,
            )
        });
    }

    fn advance_pair(&mut self) {
        self.pairs.advance();
        self.fill_pos = 0;
        self.release_pos = 0;
        self.fill_sequences.clear();
        self.release_sequences.clear();
    }

    fn swap_edits<S>(
        &self,
        index: &AssignedValueSequenceIndex,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        window: RunGapWindow,
    ) -> Vec<(usize, Option<usize>)>
    where
        S: PlanningSolution,
    {
        let mut edits = Vec::new();
        push_reassign_edits(
            &index.by_value_sequence,
            group,
            solution,
            state,
            ValueStep {
                from_value: window.source_value,
                to_value: window.target_value,
                sequence: window.fill_sequence,
            },
            &mut edits,
        );
        push_reassign_edits(
            &index.by_value_sequence,
            group,
            solution,
            state,
            ValueStep {
                from_value: window.target_value,
                to_value: window.source_value,
                sequence: window.release_sequence,
            },
            &mut edits,
        );
        edits
    }
}

fn push_reassign_edits<S>(
    by_value_sequence: &FastMap<(usize, usize), Vec<usize>>,
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &ScalarAssignmentState,
    step: ValueStep,
    edits: &mut Vec<(usize, Option<usize>)>,
) where
    S: PlanningSolution,
{
    let Some(entities) = by_value_sequence.get(&(step.from_value, step.sequence)) else {
        return;
    };
    for entity_index in entities {
        if state.current_value(*entity_index) != Some(step.to_value)
            && group.value_is_legal(solution, *entity_index, Some(step.to_value))
        {
            edits.push((*entity_index, Some(step.to_value)));
        }
    }
}

fn is_single_gap(index: &AssignedValueSequenceIndex, value: usize, sequence: usize) -> bool {
    !index.has_value(value, sequence)
        && sequence
            .checked_sub(1)
            .is_some_and(|previous| index.has_value(value, previous))
        && sequence
            .checked_add(1)
            .is_some_and(|next| index.has_value(value, next))
}

impl AssignedValueSequenceIndex {
    pub(super) fn has_value(&self, value: usize, sequence: usize) -> bool {
        self.by_value_sequence
            .get(&(value, sequence))
            .is_some_and(|entities| !entities.is_empty())
    }
}

fn is_single_island(
    by_value_sequence: &FastMap<(usize, usize), Vec<usize>>,
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
    by_value_sequence: &FastMap<(usize, usize), Vec<usize>>,
    value: usize,
    sequence: usize,
) -> bool {
    by_value_sequence
        .get(&(value, sequence))
        .is_some_and(|entities| !entities.is_empty())
}
