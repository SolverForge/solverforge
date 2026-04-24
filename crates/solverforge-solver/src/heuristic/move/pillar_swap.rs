/* PillarSwapMove - exchanges values between two pillars.

A pillar is a group of entities that share the same variable value.
This move swaps the values between two pillars atomically.

# Zero-Erasure Design

PillarSwapMove uses typed function pointers instead of `dyn Any` for complete
compile-time type safety. No runtime type checks or downcasting.
*/

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    append_canonical_usize_slice_pair, encode_option_debug, encode_usize, scoped_move_identity,
    MoveTabuScope, ScopedEntityTabuToken, TABU_OP_PILLAR_SWAP,
};
use super::{Move, MoveTabuSignature};

/// A move that swaps values between two pillars.
///
/// Stores pillar indices and typed function pointers for zero-erasure access.
/// Undo is handled by `RecordingDirector`, not by this move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The variable value type
pub struct PillarSwapMove<S, V> {
    left_indices: Vec<usize>,
    right_indices: Vec<usize>,
    descriptor_index: usize,
    variable_name: &'static str,
    // Typed getter function pointer - zero erasure.
    getter: fn(&S, usize, usize) -> Option<V>,
    // Typed setter function pointer - zero erasure.
    setter: fn(&mut S, usize, usize, Option<V>),
    variable_index: usize,
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
            variable_index: self.variable_index,
        }
    }
}

impl<S, V: Debug> Debug for PillarSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PillarSwapMove")
            .field("left_indices", &self.left_indices)
            .field("right_indices", &self.right_indices)
            .field("descriptor_index", &self.descriptor_index)
            .field("variable_index", &self.variable_index)
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
        getter: fn(&S, usize, usize) -> Option<V>,
        setter: fn(&mut S, usize, usize, Option<V>),
        variable_index: usize,
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
            variable_index,
        }
    }

    pub fn left_indices(&self) -> &[usize] {
        &self.left_indices
    }

    pub fn right_indices(&self) -> &[usize] {
        &self.right_indices
    }
}

impl<S, V> Move<S> for PillarSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
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
            .map(|&idx| (self.getter)(score_director.working_solution(), idx, self.variable_index));
        let right_val = self
            .right_indices
            .first()
            .map(|&idx| (self.getter)(score_director.working_solution(), idx, self.variable_index));

        left_val != right_val
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        // Capture all old values using typed getter - zero erasure
        let left_old: Vec<(usize, Option<V>)> = self
            .left_indices
            .iter()
            .map(|&idx| {
                (
                    idx,
                    (self.getter)(score_director.working_solution(), idx, self.variable_index),
                )
            })
            .collect();
        let right_old: Vec<(usize, Option<V>)> = self
            .right_indices
            .iter()
            .map(|&idx| {
                (
                    idx,
                    (self.getter)(score_director.working_solution(), idx, self.variable_index),
                )
            })
            .collect();

        // Get representative values for the swap
        let left_value = left_old.first().and_then(|(_, v)| v.clone());
        let right_value = right_old.first().and_then(|(_, v)| v.clone());

        // Notify before changes for all entities
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            score_director.before_variable_changed(self.descriptor_index, idx);
        }

        // Swap: left gets right's value using typed setter - zero erasure
        for &idx in &self.left_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                self.variable_index,
                right_value.clone(),
            );
        }
        // Right gets left's value
        for &idx in &self.right_indices {
            (self.setter)(
                score_director.working_solution_mut(),
                idx,
                self.variable_index,
                left_value.clone(),
            );
        }

        // Notify after changes
        for &idx in self.left_indices.iter().chain(&self.right_indices) {
            score_director.after_variable_changed(self.descriptor_index, idx);
        }

        // Register typed undo closure - restore all original values
        let setter = self.setter;
        let variable_index = self.variable_index;
        score_director.register_undo(Box::new(move |s: &mut S| {
            for (idx, old_value) in left_old {
                setter(s, idx, variable_index, old_value);
            }
            for (idx, old_value) in right_old {
                setter(s, idx, variable_index, old_value);
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

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let left_value = self.left_indices.first().and_then(|&idx| {
            (self.getter)(score_director.working_solution(), idx, self.variable_index)
        });
        let right_value = self.right_indices.first().and_then(|&idx| {
            (self.getter)(score_director.working_solution(), idx, self.variable_index)
        });
        let left_id = encode_option_debug(left_value.as_ref());
        let right_id = encode_option_debug(right_value.as_ref());
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let mut entity_ids: SmallVec<[u64; 2]> = self
            .left_indices
            .iter()
            .chain(&self.right_indices)
            .map(|&idx| encode_usize(idx))
            .collect();
        entity_ids.sort_unstable();
        entity_ids.dedup();
        let entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> = entity_ids
            .iter()
            .copied()
            .map(|entity_id| scope.entity_token(entity_id))
            .collect();

        let mut move_id = scoped_move_identity(scope, TABU_OP_PILLAR_SWAP, std::iter::empty());
        append_canonical_usize_slice_pair(&mut move_id, &self.left_indices, &self.right_indices);

        MoveTabuSignature::new(scope, move_id.clone(), move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens([
                scope.value_token(right_id),
                scope.value_token(left_id),
            ])
    }
}
