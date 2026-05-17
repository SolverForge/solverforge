use std::collections::{HashMap, HashSet};

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{rotate_entity_order, ScalarAssignmentMoveOptions};
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;

pub(super) struct AssignedValueSequenceIndex {
    pub(super) values: Vec<usize>,
    pub(super) sequence_keys: Vec<usize>,
    pub(super) by_value_sequence: HashMap<(usize, usize), Vec<usize>>,
}

pub(super) fn assigned_value_sequence_index<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &ScalarAssignmentState,
    options: ScalarAssignmentMoveOptions,
) -> AssignedValueSequenceIndex
where
    S: PlanningSolution,
{
    let mut assigned_values = HashSet::new();
    let mut sequence_keys = HashSet::new();
    let mut by_value_sequence: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

    for entity_index in 0..group.entity_count(solution) {
        let Some(value) = state.current_value(entity_index) else {
            continue;
        };
        let Some(sequence_key) = group.sequence_key(solution, entity_index, value) else {
            continue;
        };
        assigned_values.insert(value);
        sequence_keys.insert(sequence_key);
        by_value_sequence
            .entry((value, sequence_key))
            .or_default()
            .push(entity_index);
    }

    let mut values = assigned_values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    rotate_entity_order(&mut values, options.entity_offset);

    let mut sequence_keys = sequence_keys.into_iter().collect::<Vec<_>>();
    sequence_keys.sort_unstable();
    rotate_entity_order(&mut sequence_keys, options.entity_offset);

    AssignedValueSequenceIndex {
        values,
        sequence_keys,
        by_value_sequence,
    }
}
