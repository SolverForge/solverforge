use super::{adjacent_sequences, ScalarAssignmentBinding, ScalarAssignmentState};

impl ScalarAssignmentState {
    pub(in super::super) fn sort_entities_by_current_value_pressure<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entities: &mut [usize],
    ) {
        entities.sort_by_key(|entity_index| {
            let Some(value) = self.current_value(*entity_index) else {
                return (
                    usize::MAX,
                    usize::MAX,
                    group.entity_order_key(solution, *entity_index),
                    *entity_index,
                );
            };
            let load = self.assigned_count(value);
            let run = self.sequence_run_len(group, solution, *entity_index, value);
            (
                usize::MAX - run,
                usize::MAX - load,
                group.entity_order_key(solution, *entity_index),
                *entity_index,
            )
        });
    }

    pub(in super::super) fn sort_values_by_target_pressure<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        values: &mut [usize],
    ) {
        values.sort_by_key(|value| {
            (
                self.target_sequence_neighbor_count(group, solution, entity_index, *value),
                self.assigned_count(*value),
                group.value_order_key(solution, entity_index, *value),
                *value,
            )
        });
    }

    fn sequence_run_len<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> usize {
        let Some(sequence_key) = group.sequence_key(solution, entity_index, value) else {
            return 0;
        };
        let mut len = 1;
        let mut previous = sequence_key;
        while let Some(next) = previous.checked_sub(1) {
            if !self.sequence_has_value(value, next) {
                break;
            }
            len += 1;
            previous = next;
        }
        let mut next = sequence_key;
        while let Some(sequence) = next.checked_add(1) {
            if !self.sequence_has_value(value, sequence) {
                break;
            }
            len += 1;
            next = sequence;
        }
        len
    }

    fn target_sequence_neighbor_count<S>(
        &self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        entity_index: usize,
        value: usize,
    ) -> usize {
        let Some(sequence_key) = group.sequence_key(solution, entity_index, value) else {
            return 0;
        };
        adjacent_sequences(sequence_key)
            .filter(|sequence| self.sequence_has_value(value, *sequence))
            .count()
    }

    fn sequence_has_value(&self, value: usize, sequence_key: usize) -> bool {
        self.assigned_by_sequence
            .get(&(value, sequence_key))
            .is_some_and(|entities| !entities.is_empty())
    }
}
