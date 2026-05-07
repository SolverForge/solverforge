/* PillarChangeMove - assigns a value to all entities in a pillar.

A pillar is a group of entities that share the same variable value.
This move changes all of them to a new value atomically.

# Zero-Erasure Design

PillarChangeMove uses concrete function pointers instead of `dyn Any` for complete
compile-time type safety. No runtime type checks or downcasting.
*/

use std::fmt::Debug;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_debug, encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken,
};
use super::{Move, MoveTabuSignature};

/// A move that assigns a value to all entities in a pillar.
///
/// Stores entity indices and concrete function pointers for zero-erasure access.
/// Undo is handled by `RecordingDirector`, not by this move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
pub struct PillarChangeMove<S, V> {
    entity_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    to_value: Option<V>,
    // Concrete getter function pointer - zero erasure.
    getter: fn(&S, usize, usize) -> Option<V>,
    // Concrete setter function pointer - zero erasure.
    setter: fn(&mut S, usize, usize, Option<V>),
    variable_index: usize,
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
            variable_index: self.variable_index,
        }
    }
}

impl<S, V: Debug> Debug for PillarChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarChangeMove")
            .field("entity_indices", &self.entity_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
            .field("variable_name", &self.variable_name)
            .field("to_value", &self.to_value)
            .finish()
    }
}

impl<S, V> PillarChangeMove<S, V> {
    /// Creates a new pillar change move with concrete function pointers.
    ///
    /// # Arguments
    /// * `entity_indices` - Indices of entities in the pillar
    /// * `to_value` - The new value to assign to all entities
    /// * `getter` - Concrete getter function pointer
    /// * `setter` - Concrete setter function pointer
    /// * `variable_name` - Name of the variable being changed
    /// * `descriptor_index` - Index in the entity descriptor
    pub fn new(
        entity_indices: Vec<usize>,
        to_value: Option<V>,
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        variable_index: usize,
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
            variable_index,
        }
    }

    pub fn pillar_size(&self) -> usize {
        self.entity_indices.len()
    }

    pub fn to_value(&self) -> Option<&V> {
        self.to_value.as_ref()
    }
}

impl<S, V> Move<S> for PillarChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        if self.entity_indices.is_empty() {
            return false;
        }

        // Check first entity exists
        let count = score_director.entity_count(self.descriptor_index);
        if let Some(&first_idx) = self.entity_indices.first() {
            if count.is_none_or(|c| first_idx >= c) {
                return false;
            }

            // Get current value using concrete getter - zero erasure
            let current = (self.getter)(
                score_director.working_solution(),
                first_idx,
                self.variable_index,
            );

            match (&current, &self.to_value) {
                (None, None) => false,
                (Some(cur), Some(target)) => cur != target,
                _ => true,
            }
        } else {
            false
        }
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        // Capture old values using concrete getter - zero erasure
        let old_values: Vec<(usize, Option<V>)> = self
            .entity_indices
            .iter()
            .map(|&idx| {
                (
                    idx,
                    (self.getter)(score_director.working_solution(), idx, self.variable_index),
                )
            })
            .collect();

        // Notify before changes for all entities
        for &idx in &self.entity_indices {
            score_director.before_variable_changed(self.descriptor_index, idx);
        }

        // Apply new value to all entities using concrete setter - zero erasure
        for &idx in &self.entity_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                self.variable_index,
                self.to_value.clone(),
            );
        }

        // Notify after changes
        for &idx in &self.entity_indices {
            score_director.after_variable_changed(self.descriptor_index, idx);
        }

        // Register concrete undo closure
        let setter = self.setter;
        let variable_index = self.variable_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in old_values {
                setter(s, idx, variable_index, old_value);
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

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let from_value = self.entity_indices.first().and_then(|&idx| {
            (self.getter)(score_director.working_solution(), idx, self.variable_index)
        });
        let from_id = encode_option_debug(from_value.as_ref());
        let to_id = encode_option_debug(self.to_value.as_ref());
        let variable_id = hash_str(self.variable_name);
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let entity_ids: SmallVec<[u64; 2]> = self
            .entity_indices
            .iter()
            .map(|&idx| encode_usize(idx))
            .collect();
        let entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> = entity_ids
            .iter()
            .copied()
            .map(|entity_id| scope.entity_token(entity_id))
            .collect();
        let mut move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            encode_usize(self.entity_indices.len()),
            from_id,
            to_id
        ];
        move_id.extend(entity_ids.iter().copied());

        let mut undo_move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            encode_usize(self.entity_indices.len()),
            to_id,
            from_id
        ];
        undo_move_id.extend(entity_ids.iter().copied());

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens([scope.value_token(to_id)])
    }
}
