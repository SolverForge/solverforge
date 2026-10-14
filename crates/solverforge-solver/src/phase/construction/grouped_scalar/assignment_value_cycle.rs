use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_path::move_from_edits;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_coordinates::ValueCycleCoordinates;
use super::assignment_value_index::AssignedValueSequenceIndex;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use solverforge_core::fast_hash::FastSet;

pub(super) struct ValueWindowCycleCursor {
    triples: ValueCycleCoordinates,
    start_pos: usize,
    len: usize,
    max_len: usize,
}

#[derive(Clone, Copy)]
struct CycleStep {
    from_value: usize,
    to_value: usize,
    sequence_key: usize,
}

impl ValueWindowCycleCursor {
    pub(super) fn new(
        options: ScalarAssignmentMoveOptions,
        index: &AssignedValueSequenceIndex,
    ) -> Self {
        let triples = ValueCycleCoordinates::new(&index.values, options);

        let max_len = options
            .max_depth
            .max(2)
            .min(index.sequence_keys.len())
            .max(2);
        Self {
            triples,
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
        while let Some((first, second, third)) = self.triples.current(&index.values) {
            while self.start_pos < index.sequence_keys.len() {
                while self.len <= self.max_len {
                    if self.start_pos + self.len > index.sequence_keys.len() {
                        break;
                    }
                    let sequence_window =
                        &index.sequence_keys[self.start_pos..self.start_pos + self.len];
                    self.len += 1;
                    let edits = self.window_edits(
                        index,
                        group,
                        solution,
                        state,
                        (first, second, third),
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
            self.triples.advance();
            self.start_pos = 0;
            self.len = 2;
        }
        None
    }

    fn window_edits<S>(
        &self,
        index: &AssignedValueSequenceIndex,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        cycle: (usize, usize, usize),
        sequence_window: &[usize],
    ) -> Vec<(usize, Option<usize>)>
    where
        S: PlanningSolution,
    {
        let mut edits = Vec::new();
        let mut touched = FastSet::default();
        let (first, second, third) = cycle;
        for sequence_key in sequence_window {
            self.push_cycle_edits(
                index,
                &mut touched,
                &mut edits,
                group,
                solution,
                state,
                CycleStep {
                    from_value: first,
                    to_value: second,
                    sequence_key: *sequence_key,
                },
            );
            self.push_cycle_edits(
                index,
                &mut touched,
                &mut edits,
                group,
                solution,
                state,
                CycleStep {
                    from_value: second,
                    to_value: third,
                    sequence_key: *sequence_key,
                },
            );
            self.push_cycle_edits(
                index,
                &mut touched,
                &mut edits,
                group,
                solution,
                state,
                CycleStep {
                    from_value: third,
                    to_value: first,
                    sequence_key: *sequence_key,
                },
            );
        }
        edits
    }

    #[allow(clippy::too_many_arguments)]
    fn push_cycle_edits<S>(
        &self,
        index: &AssignedValueSequenceIndex,
        touched: &mut FastSet<usize>,
        edits: &mut Vec<(usize, Option<usize>)>,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        step: CycleStep,
    ) where
        S: PlanningSolution,
    {
        let Some(entities) = index
            .by_value_sequence
            .get(&(step.from_value, step.sequence_key))
        else {
            return;
        };
        for entity_index in entities {
            if touched.insert(*entity_index)
                && state.current_value(*entity_index) != Some(step.to_value)
                && group.value_is_legal(solution, *entity_index, Some(step.to_value))
            {
                edits.push((*entity_index, Some(step.to_value)));
            }
        }
    }
}
