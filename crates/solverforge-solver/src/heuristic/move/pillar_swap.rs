//! PillarSwapMove - exchanges values between two pillars.
//!
//! A pillar is a group of entities that share the same variable value.
//! This move swaps the values between two pillars atomically.
//!
//! # Zero-Erasure Design
//!
//! PillarSwapMove uses typed function pointers instead of `dyn Any` for complete
//! compile-time type safety. No runtime type checks or downcasting.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that swaps values between two pillars.
///
/// Stores pillar indices and typed function pointers for zero-erasure access.
/// Undo is handled by `RecordingScoreDirector`, not by this move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
pub struct PillarSwapMove<S, V> {
    left_indices: Vec<usize>,
    right_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    /// Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, Option<V>),
}

impl<S, V: Clone> Clone for PillarSwapMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            left_indices: self.left_indices.clone(),
            right_indices: self.right_indices.clone(),
            descriptor_index: self.descriptor_index,
            variable_name: self.variable_name,
            getter: self.getter,
            setter: self.setter,
        }
    }
}

impl<S, V: Debug> Debug for PillarSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarSwapMove")
            .field("left_indices", &self.left_indices)
            .field("right_indices", &self.right_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> PillarSwapMove<S, V> {
    /// Creates a new pillar swap move with typed function pointers.
    ///
    /// # Arguments
    /// * `left_indices` - Indices of entities in the left pillar
    /// * `right_indices` - Indices of entities in the right pillar
    /// * `getter` - Typed getter function pointer
    /// * `setter` - Typed setter function pointer
    /// * `variable_name` - Name of the variable being swapped
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        left_indices: Vec<usize>,
        right_indices: Vec<usize>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            left_indices,
            right_indices,
            descriptor_index,
            variable_name,
            getter,
            setter,
        }
    }

    /// Returns the left pillar indices.
    pub fn left_indices(&self) -> &[usize] {
        &self.left_indices
    }

    /// Returns the right pillar indices.
    pub fn right_indices(&self) -> &[usize] {
        &self.right_indices
    }
}

impl<S, V> Move<S> for PillarSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        if self.left_indices.is_empty() || self.right_indices.is_empty() {
            return false;
        }

        let count = score_director.entity_count(self.descriptor_index);
        let max = count.unwrap_or(0);

        // Check all indices valid
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            if idx >= max {
                return false;
            }
        }

        // Get representative values using typed getter - zero erasure
        let left_val = self
            .left_indices
            .first()
            .map(|&idx| (self.getter)(score_director.working_solution(), idx));
        let right_val = self
            .right_indices
            .first()
            .map(|&idx| (self.getter)(score_director.working_solution(), idx));

        left_val != right_val
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Capture all old values using typed getter - zero erasure
        let left_old: Vec<(usize, Option<V>)> = self
            .left_indices
            .iter()
            .map(|&idx| (idx, (self.getter)(score_director.working_solution(), idx)))
            .collect();
        let right_old: Vec<(usize, Option<V>)> = self
            .right_indices
            .iter()
            .map(|&idx| (idx, (self.getter)(score_director.working_solution(), idx)))
            .collect();

        // Get representative values for the swap
        let left_value = left_old.first().and_then(|(_, v)| v.clone());
        let right_value = right_old.first().and_then(|(_, v)| v.clone());

        // Notify before changes for all entities
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            score_director.before_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Swap: left gets right's value using typed setter - zero erasure
        for &idx in &self.left_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                right_value.clone(),
            );
        }
        // Right gets left's value
        for &idx in &self.right_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                left_value.clone(),
            );
        }

        // Notify after changes
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            score_director.after_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Register typed undo closure - restore all original values
        let setter = self.setter;
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in left_old {
                setter(s, idx, old_value);
            }
            for (idx, old_value) in right_old {
                setter(s, idx, old_value);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        // Return left indices as primary; caller can use left_indices/right_indices for full info
        &self.left_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}
