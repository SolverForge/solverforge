/* SublistChangeMove - relocates a contiguous sublist within or between list variables.

This move removes a range of elements from one position and inserts them at another.
Essential for vehicle routing where multiple consecutive stops need relocation.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::metadata::{
    encode_option_debug, encode_usize, hash_str, MoveTabuScope, ScopedEntityTabuToken,
    ScopedValueTabuToken,
};
use super::segment_layout::derive_segment_relocation_layout;
use super::{Move, MoveTabuSignature};

/// A move that relocates a contiguous sublist from one position to another.
///
/// Supports both intra-list moves (within same entity) and inter-list moves
/// (between different entities). Uses concrete function pointers for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::SublistChangeMove;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SoftScore;
///
/// #[derive(Clone, Debug)]
/// struct Vehicle { id: usize, visits: Vec<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { vehicles: Vec<Vehicle>, score: Option<SoftScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SoftScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Solution, entity_idx: usize) -> usize {
///     s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
/// }
/// fn list_get(s: &Solution, entity_idx: usize, pos: usize) -> Option<i32> {
///     s.vehicles
///         .get(entity_idx)
///         .and_then(|v| v.visits.get(pos))
///         .copied()
/// }
/// fn sublist_remove(s: &mut Solution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
///     s.vehicles.get_mut(entity_idx)
///         .map(|v| v.visits.drain(start..end).collect())
///         .unwrap_or_default()
/// }
/// fn sublist_insert(s: &mut Solution, entity_idx: usize, pos: usize, items: Vec<i32>) {
///     if let Some(v) = s.vehicles.get_mut(entity_idx) {
///         for (i, item) in items.into_iter().enumerate() {
///             v.visits.insert(pos + i, item);
///         }
///     }
/// }
///
/// // Move elements [1..3) from vehicle 0 to vehicle 1 at position 0
/// let m = SublistChangeMove::<Solution, i32>::new(
///     0, 1, 3,  // source: entity 0, range [1, 3)
///     1, 0,     // dest: entity 1, position 0
///     list_len, list_get, sublist_remove, sublist_insert,
///     "visits", 0,
/// );
/// ```
pub struct SublistChangeMove<S, V> {
    // Source entity index
    source_entity_index: usize,
    // Start of range in source list (inclusive)
    source_start: usize,
    // End of range in source list (exclusive)
    source_end: usize,
    // Destination entity index
    dest_entity_index: usize,
    // Position in destination list to insert at
    dest_position: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove sublist [start, end), returns removed elements
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    // Insert elements at position
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    // Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: PhantomData<fn() -> V>,
}

impl<S, V> Clone for SublistChangeMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for SublistChangeMove<S, V> {}

impl<S, V: Debug> Debug for SublistChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SublistChangeMove")
            .field("source_entity", &self.source_entity_index)
            .field("source_range", &(self.source_start..self.source_end))
            .field("dest_entity", &self.dest_entity_index)
            .field("dest_position", &self.dest_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> SublistChangeMove<S, V> {
    /* Creates a new sublist change move with concrete function pointers.

    # Arguments
    * `source_entity_index` - Entity index to remove from
    * `source_start` - Start of range (inclusive)
    * `source_end` - End of range (exclusive)
    * `dest_entity_index` - Entity index to insert into
    * `dest_position` - Position in destination list
    * `list_len` - Function to get list length
    * `sublist_remove` - Function to remove range [start, end)
    * `sublist_insert` - Function to insert elements at position
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_index: usize,
        source_start: usize,
        source_end: usize,
        dest_entity_index: usize,
        dest_position: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            source_entity_index,
            source_start,
            source_end,
            dest_entity_index,
            dest_position,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            indices: [source_entity_index, dest_entity_index],
            _phantom: PhantomData,
        }
    }

    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    pub fn source_start(&self) -> usize {
        self.source_start
    }

    pub fn source_end(&self) -> usize {
        self.source_end
    }

    pub fn sublist_len(&self) -> usize {
        self.source_end.saturating_sub(self.source_start)
    }

    pub fn dest_entity_index(&self) -> usize {
        self.dest_entity_index
    }

    pub fn dest_position(&self) -> usize {
        self.dest_position
    }

    pub fn is_intra_list(&self) -> bool {
        self.source_entity_index == self.dest_entity_index
    }
}

impl<S, V> Move<S> for SublistChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Check range is valid (start < end)
        if self.source_start >= self.source_end {
            return false;
        }

        // Check source range is within bounds
        let source_len = (self.list_len)(solution, self.source_entity_index);
        if self.source_end > source_len {
            return false;
        }

        // Check destination position is valid
        let dest_len = (self.list_len)(solution, self.dest_entity_index);
        let sublist_len = self.sublist_len();

        let max_dest = if self.is_intra_list() {
            // After removing sublist, list is shorter
            source_len - sublist_len
        } else {
            dest_len
        };

        if self.dest_position > max_dest {
            return false;
        }

        // For intra-list, dest_position is relative to the post-removal list.
        if self.is_intra_list() {
            // Re-inserting at the original start position is the only no-op.
            if self.dest_position == self.source_start {
                return false;
            }
        }

        true
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        let layout = derive_segment_relocation_layout(
            self.source_entity_index,
            self.source_start,
            self.source_end,
            self.dest_entity_index,
            self.dest_position,
        );

        // Notify before changes
        score_director.before_variable_changed(self.descriptor_index, self.source_entity_index);
        if !self.is_intra_list() {
            score_director.before_variable_changed(self.descriptor_index, self.dest_entity_index);
        }

        // Remove sublist from source
        let elements = (self.sublist_remove)(
            score_director.working_solution_mut(),
            self.source_entity_index,
            self.source_start,
            self.source_end,
        );

        // dest_position is relative to post-removal list, no adjustment needed
        let dest_pos = layout.exact.dest_position;

        // Insert at destination
        (self.sublist_insert)(
            score_director.working_solution_mut(),
            self.dest_entity_index,
            dest_pos,
            elements,
        );

        // Notify after changes
        score_director.after_variable_changed(self.descriptor_index, self.source_entity_index);
        if !self.is_intra_list() {
            score_director.after_variable_changed(self.descriptor_index, self.dest_entity_index);
        }
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        let inverse = derive_segment_relocation_layout(
            self.source_entity_index,
            self.source_start,
            self.source_end,
            self.dest_entity_index,
            self.dest_position,
        )
        .inverse;
        score_director.before_variable_changed(self.descriptor_index, inverse.source_entity_index);
        if inverse.source_entity_index != inverse.dest_entity_index {
            score_director
                .before_variable_changed(self.descriptor_index, inverse.dest_entity_index);
        }
        let removed = (self.sublist_remove)(
            score_director.working_solution_mut(),
            inverse.source_entity_index,
            inverse.source_range.start,
            inverse.source_range.end,
        );
        (self.sublist_insert)(
            score_director.working_solution_mut(),
            inverse.dest_entity_index,
            inverse.dest_position,
            removed,
        );
        score_director.after_variable_changed(self.descriptor_index, inverse.source_entity_index);
        if inverse.source_entity_index != inverse.dest_entity_index {
            score_director.after_variable_changed(self.descriptor_index, inverse.dest_entity_index);
        }
    }

    fn descriptor_index(&self) -> usize {
        self.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        if self.is_intra_list() {
            &self.indices[0..1]
        } else {
            &self.indices
        }
    }

    fn variable_name(&self) -> &str {
        self.variable_name
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        let layout = derive_segment_relocation_layout(
            self.source_entity_index,
            self.source_start,
            self.source_end,
            self.dest_entity_index,
            self.dest_position,
        );
        let mut moved_ids: SmallVec<[u64; 2]> = SmallVec::new();
        for pos in self.source_start..self.source_end {
            let value = (self.list_get)(
                score_director.working_solution(),
                self.source_entity_index,
                pos,
            );
            moved_ids.push(encode_option_debug(value.as_ref()));
        }
        let source_entity_id = encode_usize(self.source_entity_index);
        let dest_entity_id = encode_usize(self.dest_entity_index);
        let variable_id = hash_str(self.variable_name);
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
            smallvec![scope.entity_token(source_entity_id)];
        if !self.is_intra_list() {
            entity_tokens.push(scope.entity_token(dest_entity_id));
        }
        let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = moved_ids
            .iter()
            .copied()
            .map(|value_id| scope.value_token(value_id))
            .collect();
        let mut move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            source_entity_id,
            encode_usize(layout.exact.source_range.start),
            encode_usize(layout.exact.source_range.end),
            dest_entity_id,
            encode_usize(layout.exact.dest_position)
        ];
        move_id.extend(moved_ids.iter().copied());
        let mut undo_move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            encode_usize(layout.inverse.source_entity_index),
            encode_usize(layout.inverse.source_range.start),
            encode_usize(layout.inverse.source_range.end),
            encode_usize(layout.inverse.dest_entity_index),
            encode_usize(layout.inverse.dest_position)
        ];
        undo_move_id.extend(moved_ids.iter().copied());

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens(destination_value_tokens)
    }
}
