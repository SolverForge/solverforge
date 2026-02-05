//! PillarChangeMove - assigns a value to all entities in a pillar.
//!
//! A pillar is a group of entities that share the same variable value.
//! This move changes all of them to a new value atomically.
//!
//! # Zero-Erasure Design
//!
//! PillarChangeMove uses typed function pointers instead of `dyn Any` for complete
//! compile-time type safety. No runtime type checks or downcasting.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that assigns a value to all entities in a pillar.
///
/// Stores entity indices and typed function pointers for zero-erasure access.
/// Undo is handled by `RecordingScoreDirector`, not by this move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
pub struct PillarChangeMove<S, V> {
    entity_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    to_value: Option<V>,
    /// Typed getter function pointer - zero erasure.
    getter: fn(&S, usize) -> Option<V>,
    /// Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, Option<V>),
}

impl<S, V: Clone> Clone for PillarChangeMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_indices: self.entity_indices.clone(),
            descriptor_index: self.descriptor_index,
            variable_name: self.variable_name,
            to_value: self.to_value.clone(),
            getter: self.getter,
            setter: self.setter,
        }
    }
}

impl<S, V: Debug> Debug for PillarChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarChangeMove")
            .field("entity_indices", &self.entity_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S, V> PillarChangeMove<S, V> {
    /// Creates a new pillar change move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities in the pillar
    /// * `to_value` - The new value to assign to all entities
    /// * `getter` - Typed getter function pointer
    /// * `setter` - Typed setter function pointer
    /// * `variable_name` - Name of the variable being changed
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        entity_indices: Vec<usize>,
        to_value: Option<V>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_indices,
            descriptor_index,
            variable_name,
            to_value,
            getter,
            setter,
        }
    }

    /// Returns the pillar size.
    pub fn pillar_size(&self) -> usize {
        self.entity_indices.len()
    }

    /// Returns the target value.
    pub fn to_value(&self) -> Option<&V> {
        self.to_value.as_ref()
    }
}

impl<S, V> Move<S> for PillarChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        if self.entity_indices.is_empty() {
            return false;
        }

        // Check first entity exists
        let count = score_director.entity_count(self.descriptor_index);
        if let Some(&first_idx) = self.entity_indices.first() {
            if count.is_none_or(|c| first_idx >= c) {
                return false;
            }

            // Get current value using typed getter - zero erasure
            let current = (self.getter)(score_director.working_solution(), first_idx);

            match (&current, &self.to_value) {
                (None, None) => false,
                (Some(cur), Some(target)) => cur != target,
                _ => true,
            }
        } else {
            false
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Capture old values using typed getter - zero erasure
        let old_values: Vec<(usize, Option<V>)> = self
            .entity_indices
            .iter()
            .map(|&idx| (idx, (self.getter)(score_director.working_solution(), idx)))
            .collect();

        // Notify before changes for all entities
        for &idx in &self.entity_indices {
            score_director.before_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Apply new value to all entities using typed setter - zero erasure
        for &idx in &self.entity_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                self.to_value.clone(),
            );
        }

        // Notify after changes
        for &idx in &self.entity_indices {
            score_director.after_variable_changed(self.descriptor_index, idx, self.variable_name);
        }

        // Register typed undo closure
        let setter = self.setter;
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in old_values {
                setter(s, idx, old_value);
            }
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}
