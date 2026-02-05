//! Lazy iterator for move generation.

use std::sync::Arc;

use super::{get_range_values, DynamicChangeMove};
use crate::solution::{DynamicSolution, DynamicValue};

/// Lazy iterator over all possible change moves for a dynamic solution.
///
/// This iterator generates moves on-demand rather than pre-computing them all,
/// which can significantly reduce memory usage for large solutions with many
/// entities and value ranges.
///
/// # Performance Characteristics
///
/// Benchmarks comparing lazy iteration vs eager Vec collection show:
///
/// **Full iteration (all moves consumed):**
/// - Small scale (1K moves): ~equal performance
/// - Medium scale (10K-50K moves): lazy is ~10-20% faster
/// - Large scale (1M moves): lazy is ~2x faster due to avoiding allocation overhead
///
/// **Partial iteration (only first N moves consumed):**
/// This is where lazy iteration really shines. For a 50K move space:
/// - take(10): ~3500x faster (avoid generating 99.98% of moves)
/// - take(100): ~570x faster
/// - take(1000): ~60x faster
/// - take(10000): ~6x faster
///
/// The lazy iterator is particularly beneficial when:
/// 1. Only a subset of moves are consumed (e.g., `FirstAcceptedForager` stops after finding improvement)
/// 2. Memory allocation overhead is significant (large move counts)
/// 3. Early termination is common in the consuming algorithm
///
/// # Iteration Order
///
/// The iterator traverses in the following order:
/// 1. Entity classes (by class_idx)
/// 2. Entities within each class (by entity_idx)
/// 3. Planning variables within each entity (by field_idx)
/// 4. Values in the value range for each variable (by value_idx)
pub struct DynamicMoveIterator<'a> {
    solution: &'a DynamicSolution,
    // Current entity class index
    class_idx: usize,
    // Current entity index within the class
    entity_idx: usize,
    // Current index into planning_variable_indices for the class
    var_slot_idx: usize,
    // Current value index for the current variable's value range
    value_idx: usize,
    // Cached values for the current variable's range (avoids repeated allocation)
    current_values: Vec<DynamicValue>,
    // Cached field_idx for current variable
    current_field_idx: usize,
    // Cached variable name for current variable
    current_variable_name: Arc<str>,
    // Whether we've finished iterating
    exhausted: bool,
}

impl<'a> DynamicMoveIterator<'a> {
    /// Creates a new move iterator for the given solution.
    pub fn new(solution: &'a DynamicSolution) -> Self {
        let mut iter = Self {
            solution,
            class_idx: 0,
            entity_idx: 0,
            var_slot_idx: 0,
            value_idx: 0,
            current_values: Vec::new(),
            current_field_idx: 0,
            current_variable_name: Arc::from(""),
            exhausted: false,
        };
        // Initialize to the first valid position
        iter.advance_to_valid_position();
        iter
    }

    // Advances the iterator state to the next valid position that has moves.
    // Returns true if a valid position was found, false if exhausted.
    fn advance_to_valid_position(&mut self) -> bool {
        loop {
            // Check if we've exhausted all classes
            if self.class_idx >= self.solution.descriptor.entity_classes.len() {
                self.exhausted = true;
                return false;
            }

            let class_def = &self.solution.descriptor.entity_classes[self.class_idx];
            let entity_count = self
                .solution
                .entities
                .get(self.class_idx)
                .map(|e| e.len())
                .unwrap_or(0);

            // Check if we've exhausted entities in this class
            if self.entity_idx >= entity_count {
                self.class_idx += 1;
                self.entity_idx = 0;
                self.var_slot_idx = 0;
                self.value_idx = 0;
                continue;
            }

            // Check if we've exhausted planning variables for this entity
            if self.var_slot_idx >= class_def.planning_variable_indices.len() {
                self.entity_idx += 1;
                self.var_slot_idx = 0;
                self.value_idx = 0;
                continue;
            }

            // Get the current field info
            let field_idx = class_def.planning_variable_indices[self.var_slot_idx];
            let field_def = &class_def.fields[field_idx];

            // Check if this variable has a value range
            let Some(range_name) = &field_def.value_range else {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            };

            // Check if the range exists
            let Some(range) = self.solution.descriptor.value_range(range_name) else {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            };

            // Get the values for this range
            let values = get_range_values(range, self.solution);
            if values.is_empty() {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            }

            // Check if we've exhausted values for this variable
            if self.value_idx >= values.len() {
                self.var_slot_idx += 1;
                self.value_idx = 0;
                continue;
            }

            // We have a valid position - cache the current variable info
            self.current_values = values;
            self.current_field_idx = field_idx;
            self.current_variable_name = field_def.name.clone();
            return true;
        }
    }
}

impl<'a> Iterator for DynamicMoveIterator<'a> {
    type Item = DynamicChangeMove;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        // Create the move for the current position
        let m = DynamicChangeMove::new(
            self.class_idx,
            self.entity_idx,
            self.current_field_idx,
            self.current_variable_name.clone(),
            self.current_values[self.value_idx].clone(),
        );

        // Advance to the next value
        self.value_idx += 1;

        // If we've exhausted values for this variable, advance to the next valid position
        if self.value_idx >= self.current_values.len() {
            self.var_slot_idx += 1;
            self.value_idx = 0;
            self.advance_to_valid_position();
        }

        Some(m)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // We can compute the exact size but it requires iterating through the structure
        // For now, provide a lower bound of 0 and no upper bound
        (0, None)
    }
}
