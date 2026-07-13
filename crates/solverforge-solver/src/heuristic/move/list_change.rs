/* ListChangeMove - relocates an element within or between list variables.

This move removes an element from one position and inserts it at another.
Essential for vehicle routing and scheduling problems.

# Zero-Erasure Design

Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.
*/

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    change_candidate_trace_identity, change_do_move, change_is_doable, change_tabu_signature,
    change_undo_move, ChangeCoordinates, ChangeValueTransfer, StaticListChangeAccess,
};
use super::{Move, MoveTabuSignature};

/// A move that relocates an element from one list position to another.
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
/// use solverforge_solver::heuristic::r#move::ListChangeMove;
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
/// fn list_remove(s: &mut Solution, entity_idx: usize, pos: usize) -> Option<i32> {
///     s.vehicles.get_mut(entity_idx).map(|v| v.visits.remove(pos))
/// }
/// fn list_insert(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
///     if let Some(v) = s.vehicles.get_mut(entity_idx) { v.visits.insert(pos, val); }
/// }
///
/// // Move element from vehicle 0 position 2 to vehicle 1 position 0
/// let m = ListChangeMove::<Solution, i32>::new(
///     0, 2, 1, 0,
///     list_len, list_get, list_remove, list_insert,
///     "visits", 0,
/// );
/// ```
pub struct ListChangeMove<S, V> {
    // Source entity index (which entity's list to remove from)
    source_entity_index: usize,
    // Position in source list to remove from
    source_position: usize,
    // Destination entity index (which entity's list to insert into)
    dest_entity_index: usize,
    // Position in destination list to insert at
    dest_position: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove element at position, returns removed value
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    // Insert element at position
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    // Store indices for entity_indices()
    indices: [usize; 2],
}

impl<S, V> Clone for ListChangeMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for ListChangeMove<S, V> {}

impl<S, V: Debug> Debug for ListChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListChangeMove")
            .field("source_entity", &self.source_entity_index)
            .field("source_position", &self.source_position)
            .field("dest_entity", &self.dest_entity_index)
            .field("dest_position", &self.dest_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> ListChangeMove<S, V> {
    /* Creates a new list change move with concrete function pointers.

    # Arguments
    * `source_entity_index` - Entity index to remove from
    * `source_position` - Position in source list
    * `dest_entity_index` - Entity index to insert into
    * `dest_position` - Position in destination list
    * `list_len` - Function to get list length
    * `list_remove` - Function to remove element at position
    * `list_insert` - Function to insert element at position
    * `variable_name` - Name of the list variable
    * `descriptor_index` - Entity descriptor index
    */
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_index: usize,
        source_position: usize,
        dest_entity_index: usize,
        dest_position: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            source_entity_index,
            source_position,
            dest_entity_index,
            dest_position,
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            indices: [source_entity_index, dest_entity_index],
        }
    }

    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    pub fn source_position(&self) -> usize {
        self.source_position
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

    fn access(&self) -> StaticListChangeAccess<S, V> {
        StaticListChangeAccess {
            list_len: self.list_len,
            list_get: self.list_get,
            list_remove: self.list_remove,
            list_insert: self.list_insert,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
        }
    }

    fn coordinates(&self) -> ChangeCoordinates {
        ChangeCoordinates {
            source_entity: self.source_entity_index,
            source_position: self.source_position,
            destination_entity: self.dest_entity_index,
            destination_position: self.dest_position,
        }
    }
}

impl<S, V> Move<S> for ListChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        change_is_doable(&self.access(), self.coordinates(), score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        change_do_move(
            &self.access(),
            self.coordinates(),
            ChangeValueTransfer::CloneBeforeInsert,
            score_director,
        );
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        change_undo_move(&self.access(), self.coordinates(), score_director);
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
        "list_change"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        change_tabu_signature(&self.access(), self.coordinates(), score_director)
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(change_candidate_trace_identity(
            &self.access(),
            self.coordinates(),
        ))
    }
}
