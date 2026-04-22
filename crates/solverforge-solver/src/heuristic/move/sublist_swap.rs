/* SublistSwapMove - swaps two contiguous sublists within or between list variables.

This move exchanges two ranges of elements. Essential for vehicle routing
where segments need to be swapped between vehicles.

# Zero-Erasure Design

Uses typed function pointers for list operations. No `dyn Any`, no downcasting.
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
use super::segment_layout::derive_segment_swap_layout;
use super::{Move, MoveTabuSignature};

/// A move that swaps two contiguous sublists.
///
/// Supports both intra-list swaps (within same entity) and inter-list swaps
/// (between different entities). Uses typed function pointers for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
///
/// # Example
///
/// ```
/// use solverforge_solver::heuristic::r#move::SublistSwapMove;
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
/// // Swap [1..3) from vehicle 0 with [0..2) from vehicle 1
/// let m = SublistSwapMove::<Solution, i32>::new(
///     0, 1, 3,  // first: entity 0, range [1, 3)
///     1, 0, 2,  // second: entity 1, range [0, 2)
///     list_len, list_get, sublist_remove, sublist_insert,
///     "visits", 0,
/// );
/// ```
pub struct SublistSwapMove<S, V> {
    // First entity index
    first_entity_index: usize,
    // Start of first range (inclusive)
    first_start: usize,
    // End of first range (exclusive)
    first_end: usize,
    // Second entity index
    second_entity_index: usize,
    // Start of second range (inclusive)
    second_start: usize,
    // End of second range (exclusive)
    second_end: usize,
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

impl<S, V> Clone for SublistSwapMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for SublistSwapMove<S, V> {}

impl<S, V: Debug> Debug for SublistSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SublistSwapMove")
            .field("first_entity", &self.first_entity_index)
            .field("first_range", &(self.first_start..self.first_end))
            .field("second_entity", &self.second_entity_index)
            .field("second_range", &(self.second_start..self.second_end))
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> SublistSwapMove<S, V> {
    /* Creates a new sublist swap move with typed function pointers.

    # Arguments
    * `first_entity_index` - First entity index
    * `first_start` - Start of first range (inclusive)
    * `first_end` - End of first range (exclusive)
    * `second_entity_index` - Second entity index
    * `second_start` - Start of second range (inclusive)
    * `second_end` - End of second range (exclusive)
    * `list_len` - Function to get list length
    * `sublist_remove` - Function to remove range [start, end)
    * `sublist_insert` - Function to insert elements at position
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_entity_index: usize,
        first_start: usize,
        first_end: usize,
        second_entity_index: usize,
        second_start: usize,
        second_end: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_start,
            first_end,
            second_entity_index,
            second_start,
            second_end,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            indices: [first_entity_index, second_entity_index],
            _phantom: PhantomData,
        }
    }

    pub fn first_entity_index(&self) -> usize {
        self.first_entity_index
    }

    pub fn first_start(&self) -> usize {
        self.first_start
    }

    pub fn first_end(&self) -> usize {
        self.first_end
    }

    pub fn first_len(&self) -> usize {
        self.first_end.saturating_sub(self.first_start)
    }

    pub fn second_entity_index(&self) -> usize {
        self.second_entity_index
    }

    pub fn second_start(&self) -> usize {
        self.second_start
    }

    pub fn second_end(&self) -> usize {
        self.second_end
    }

    pub fn second_len(&self) -> usize {
        self.second_end.saturating_sub(self.second_start)
    }

    pub fn is_intra_list(&self) -> bool {
        self.first_entity_index == self.second_entity_index
    }
}

impl<S, V> Move<S> for SublistSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Both ranges must be valid (start < end)
        if self.first_start >= self.first_end || self.second_start >= self.second_end {
            return false;
        }

        // Check first range is within bounds
        let first_list_len = (self.list_len)(solution, self.first_entity_index);
        if self.first_end > first_list_len {
            return false;
        }

        // Check second range is within bounds
        let second_list_len = (self.list_len)(solution, self.second_entity_index);
        if self.second_end > second_list_len {
            return false;
        }

        // For intra-list swaps, ranges must not overlap
        if self.is_intra_list() {
            // Ranges overlap if one starts before the other ends
            let overlaps = self.first_start < self.second_end && self.second_start < self.first_end;
            if overlaps {
                return false;
            }
        }

        true
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) {
        let layout = derive_segment_swap_layout(
            self.first_entity_index,
            self.first_start,
            self.first_end,
            self.second_entity_index,
            self.second_start,
            self.second_end,
        );

        // Notify before changes
        score_director.before_variable_changed(self.descriptor_index, self.first_entity_index);
        if !self.is_intra_list() {
            score_director.before_variable_changed(self.descriptor_index, self.second_entity_index);
        }

        if self.is_intra_list() {
            let (early_range, late_range) = layout.exact.ordered_ranges();

            // Remove later range first
            let late_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                late_range.start,
                late_range.end,
            );

            // Remove earlier range
            let early_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                early_range.start,
                early_range.end,
            );

            // Insert late elements at early position
            let late_len = late_range.len();
            let early_len = early_range.len();
            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                early_range.start,
                late_elements,
            );

            /* Insert early elements at adjusted late position
            After removing early range, late_start shifts by early_len
            After inserting late elements, it shifts back by late_len
            */
            let new_late_pos = late_range.start - early_len + late_len;
            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                new_late_pos,
                early_elements,
            );

            // Register undo - swap back
            let sublist_remove = self.sublist_remove;
            let sublist_insert = self.sublist_insert;
            let entity = self.first_entity_index;
            let inverse = layout.inverse;

            score_director.register_undo(Box::new(move |s: &mut S| {
                let (early_range, late_range) = inverse.ordered_ranges();
                let late_elements = sublist_remove(s, entity, late_range.start, late_range.end);
                let early_elements = sublist_remove(s, entity, early_range.start, early_range.end);
                let late_len = late_range.len();
                let early_len = early_range.len();
                sublist_insert(s, entity, early_range.start, late_elements);
                let new_late_pos = late_range.start - early_len + late_len;
                sublist_insert(s, entity, new_late_pos, early_elements);
            }));
        } else {
            // Inter-list swap: simpler, no index interaction between lists
            let first_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                self.first_start,
                self.first_end,
            );

            let second_elements = (self.sublist_remove)(
                score_director.working_solution_mut(),
                self.second_entity_index,
                self.second_start,
                self.second_end,
            );

            // Insert swapped
            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.first_entity_index,
                self.first_start,
                second_elements,
            );

            (self.sublist_insert)(
                score_director.working_solution_mut(),
                self.second_entity_index,
                self.second_start,
                first_elements,
            );

            // Register undo
            let sublist_remove = self.sublist_remove;
            let sublist_insert = self.sublist_insert;
            let inverse = layout.inverse;

            score_director.register_undo(Box::new(move |s: &mut S| {
                let first_elements = sublist_remove(
                    s,
                    inverse.first_entity_index,
                    inverse.first_range.start,
                    inverse.first_range.end,
                );
                let second_elements = sublist_remove(
                    s,
                    inverse.second_entity_index,
                    inverse.second_range.start,
                    inverse.second_range.end,
                );
                sublist_insert(
                    s,
                    inverse.first_entity_index,
                    inverse.first_range.start,
                    second_elements,
                );
                sublist_insert(
                    s,
                    inverse.second_entity_index,
                    inverse.second_range.start,
                    first_elements,
                );
            }));
        }

        // Notify after changes
        score_director.after_variable_changed(self.descriptor_index, self.first_entity_index);
        if !self.is_intra_list() {
            score_director.after_variable_changed(self.descriptor_index, self.second_entity_index);
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
        let layout = derive_segment_swap_layout(
            self.first_entity_index,
            self.first_start,
            self.first_end,
            self.second_entity_index,
            self.second_start,
            self.second_end,
        );
        let mut first_value_ids: SmallVec<[u64; 2]> = SmallVec::new();
        for pos in self.first_start..self.first_end {
            let value = (self.list_get)(
                score_director.working_solution(),
                self.first_entity_index,
                pos,
            );
            first_value_ids.push(encode_option_debug(value.as_ref()));
        }
        let mut second_value_ids: SmallVec<[u64; 2]> = SmallVec::new();
        for pos in self.second_start..self.second_end {
            let value = (self.list_get)(
                score_director.working_solution(),
                self.second_entity_index,
                pos,
            );
            second_value_ids.push(encode_option_debug(value.as_ref()));
        }
        let first_entity_id = encode_usize(self.first_entity_index);
        let second_entity_id = encode_usize(self.second_entity_index);
        let variable_id = hash_str(self.variable_name);
        let scope = MoveTabuScope::new(self.descriptor_index, self.variable_name);
        let mut entity_tokens: SmallVec<[ScopedEntityTabuToken; 2]> =
            smallvec![scope.entity_token(first_entity_id)];
        if !self.is_intra_list() {
            entity_tokens.push(scope.entity_token(second_entity_id));
        }
        let destination_value_tokens: SmallVec<[ScopedValueTabuToken; 2]> = first_value_ids
            .iter()
            .chain(second_value_ids.iter())
            .copied()
            .map(|value_id| scope.value_token(value_id))
            .collect();
        let mut move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            encode_usize(layout.exact.first_entity_index),
            encode_usize(layout.exact.first_range.start),
            encode_usize(layout.exact.first_range.end),
            encode_usize(layout.exact.second_entity_index),
            encode_usize(layout.exact.second_range.start),
            encode_usize(layout.exact.second_range.end)
        ];
        move_id.extend(first_value_ids.iter().copied());
        move_id.extend(second_value_ids.iter().copied());
        let mut undo_move_id = smallvec![
            encode_usize(self.descriptor_index),
            variable_id,
            encode_usize(layout.inverse.first_entity_index),
            encode_usize(layout.inverse.first_range.start),
            encode_usize(layout.inverse.first_range.end),
            encode_usize(layout.inverse.second_entity_index),
            encode_usize(layout.inverse.second_range.start),
            encode_usize(layout.inverse.second_range.end)
        ];
        undo_move_id.extend(second_value_ids.iter().copied());
        undo_move_id.extend(first_value_ids.iter().copied());

        MoveTabuSignature::new(scope, move_id, undo_move_id)
            .with_entity_tokens(entity_tokens)
            .with_destination_value_tokens(destination_value_tokens)
    }
}
