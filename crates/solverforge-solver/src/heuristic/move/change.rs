//! ChangeMove - assigns a value to a planning variable.
//!
//! This is the most fundamental move type. It takes a value and assigns
//! it to a planning variable on an entity.
//!
//! # Zero-Erasure Design
//!
//! This move stores typed function pointers that operate directly on
//! the solution. No `Arc<dyn>`, no `Box<dyn Any>`, no `downcast_ref`.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that assigns a value to an entity's variable.
///
/// Stores typed function pointers for zero-erasure execution.
/// No trait objects, no boxing - all operations are fully typed at compile time.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
pub struct ChangeMove<S, V> {
    entity_index: usize,
    to_value: Option<V>,
    getter: fn(&S, usize) -> Option<V>,
    setter: fn(&mut S, usize, Option<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V: Clone> Clone for ChangeMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            to_value: self.to_value.clone(),
            getter: self.getter,
            setter: self.setter,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }
}

impl<S, V: Copy> Copy for ChangeMove<S, V> {}

impl<S, V: Debug> Debug for ChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeMove")
            .field("entity_index", &self.entity_index)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S, V> ChangeMove<S, V> {
    /// Creates a new change move with typed function pointers.
    ///
    /// # Arguments
    /// * `entity_index` - Index of the entity in its collection
    /// * `to_value` - The value to assign (None to unassign)
    /// * `getter` - Function pointer to get current value from solution
    /// * `setter` - Function pointer to set value on solution
    /// * `variable_name` - Name of the variable (for debugging)
    /// * `descriptor_index` - Index of the entity descriptor
    pub fn new(
        entity_index: usize,
        to_value: Option<V>,
        getter: fn(&S, usize) -> Option<V>,
        setter: fn(&mut S, usize, Option<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_index,
            to_value,
            getter,
            setter,
            variable_name,
            descriptor_index,
        }
    }

    /// Returns the entity index.
    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    /// Returns the target value.
    pub fn to_value(&self) -> Option<&V> {
        self.to_value.as_ref()
    }

    /// Returns the getter function pointer.
    pub fn getter(&self) -> fn(&S, usize) -> Option<V> {
        self.getter
    }

    /// Returns the setter function pointer.
    pub fn setter(&self) -> fn(&mut S, usize, Option<V>) {
        self.setter
    }
}

impl<S, V> Move<S> for ChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        // Get current value using typed getter - no boxing, no downcast
        let current = (self.getter)(score_director.working_solution(), self.entity_index);

        // Compare directly - fully typed comparison
        match (&current, &self.to_value) {
            (None, None) => false,                      // Both unassigned
            (Some(cur), Some(target)) => cur != target, // Different values
            _ => true,                                  // One assigned, one not
        }
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Capture old value using typed getter - zero erasure
        let old_value = (self.getter)(score_director.working_solution(), self.entity_index);

        // Notify before change
        score_director.before_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Set value using typed setter - no boxing
        (self.setter)(
            score_director.working_solution_mut(),
            self.entity_index,
            self.to_value.clone(),
        );

        // Notify after change
        score_director.after_variable_changed(
            self.descriptor_index,
            self.entity_index,
            self.variable_name,
        );

        // Register typed undo closure - zero erasure
        let setter = self.setter;
        let idx = self.entity_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            setter(s, idx, old_value);
        }));
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        std::slice::from_ref(&self.entity_index)
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }
}
