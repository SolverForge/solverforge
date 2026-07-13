/* ListSwapMove - swaps two elements within or between list variables.

This move exchanges two elements at different positions.
Useful for TSP-style improvements and route optimization.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    swap_candidate_trace_identity, swap_do_move, swap_is_doable, swap_tabu_signature,
    StaticListSwapAccess, SwapCoordinates,
};
use super::{Move, MoveTabuSignature};

/// A move that swaps two elements in list variables.
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
/// use solverforge_solver::heuristic::r#move::ListSwapMove;
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
///     s.vehicles.get(entity_idx).and_then(|v| v.visits.get(pos).copied())
/// }
/// fn list_set(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
///     if let Some(v) = s.vehicles.get_mut(entity_idx) {
///         if let Some(elem) = v.visits.get_mut(pos) { *elem = val; }
///     }
/// }
///
/// // Swap elements at positions 1 and 3 in vehicle 0
/// let m = ListSwapMove::<Solution, i32>::new(
///     0, 1, 0, 3,
///     list_len, list_get, list_set,
///     "visits", 0,
/// );
/// ```
pub struct ListSwapMove<S, V> {
    // First entity index
    first_entity_index: usize,
    // Position in first entity's list
    first_position: usize,
    // Second entity index
    second_entity_index: usize,
    // Position in second entity's list
    second_position: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Set element at position
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    // Store indices for entity_indices()
    indices: [usize; 2],
}

impl<S, V> Clone for ListSwapMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for ListSwapMove<S, V> {}

impl<S, V: Debug> Debug for ListSwapMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListSwapMove")
            .field("first_entity", &self.first_entity_index)
            .field("first_position", &self.first_position)
            .field("second_entity", &self.second_entity_index)
            .field("second_position", &self.second_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListSwapMove<S, V> {
    /* Creates a new list swap move with concrete function pointers.

    # Arguments
    * `first_entity_index` - First entity index
    * `first_position` - Position in first entity's list
    * `second_entity_index` - Second entity index
    * `second_position` - Position in second entity's list
    * `list_len` - Function to get list length
    * `list_get` - Function to get element at position
    * `list_set` - Function to set element at position
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        first_entity_index: usize,
        first_position: usize,
        second_entity_index: usize,
        second_position: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            first_entity_index,
            first_position,
            second_entity_index,
            second_position,
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            indices: [first_entity_index, second_entity_index],
        }
    }

    pub fn first_entity_index(&self) -> usize {
        self.first_entity_index
    }

    pub fn first_position(&self) -> usize {
        self.first_position
    }

    pub fn second_entity_index(&self) -> usize {
        self.second_entity_index
    }

    pub fn second_position(&self) -> usize {
        self.second_position
    }

    pub fn is_intra_list(&self) -> bool {
        self.first_entity_index == self.second_entity_index
    }

    fn access(&self) -> StaticListSwapAccess<S, V> {
        StaticListSwapAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            list_set: self.list_set,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }

    fn coordinates(&self) -> SwapCoordinates {
        SwapCoordinates {
            first_entity: self.first_entity_index,
            first_position: self.first_position,
            second_entity: self.second_entity_index,
            second_position: self.second_position,
        }
    }
}

impl<S, V> Move<S> for ListSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        swap_is_doable(&self.access(), self.coordinates(), score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        swap_do_move(&self.access(), self.coordinates(), score_director);
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        swap_do_move(&self.access(), self.coordinates(), score_director);
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
        "list_swap"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        swap_tabu_signature(&self.access(), self.coordinates(), score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(swap_candidate_trace_identity(
            &self.access(),
            self.coordinates(),
        ))
    }
}
