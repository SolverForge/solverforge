/* SublistSwapMove - swaps two contiguous sublists within or between list variables.

This move exchanges two ranges of elements. Essential for vehicle routing
where segments need to be swapped between vehicles.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    sublist_swap_candidate_trace_identity, sublist_swap_do_move, sublist_swap_is_doable,
    sublist_swap_tabu_signature, sublist_swap_undo_move, StaticListWindowAccess,
};
use super::segment_layout::SegmentSwapCoords;
use super::{Move, MoveTabuSignature};

/// A move that swaps two contiguous sublists.
///
/// Supports both intra-list swaps (within same entity) and inter-list swaps
/// (between different entities). Uses concrete function pointers for zero-erasure.
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
    /* Creates a new sublist swap move with concrete function pointers.

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

    fn access(&self) -> StaticListWindowAccess<S, V> {
        StaticListWindowAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            sublist_remove: self.sublist_remove,
            sublist_insert: self.sublist_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }

    fn coordinates(&self) -> SegmentSwapCoords {
        SegmentSwapCoords::new(
            self.first_entity_index,
            self.first_start,
            self.first_end,
            self.second_entity_index,
            self.second_start,
            self.second_end,
        )
    }
}

impl<S, V> Move<S> for SublistSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        sublist_swap_is_doable(&self.access(), self.coordinates(), score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        sublist_swap_do_move(&self.access(), self.coordinates(), score_director);
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        sublist_swap_undo_move(&self.access(), self.coordinates(), score_director);
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

    fn telemetry_label(&self) -> &'static str {
        "sublist_swap"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        sublist_swap_tabu_signature(&self.access(), self.coordinates(), score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(sublist_swap_candidate_trace_identity(
            &self.access(),
            self.coordinates(),
        ))
    }
}
