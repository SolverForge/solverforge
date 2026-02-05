//! ListChangeMove - relocates an element within or between list variables.
//!
//! This move removes an element from one position and inserts it at another.
//! Essential for vehicle routing and scheduling problems.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that relocates an element from one list position to another.
///
/// Supports both intra-list moves (within same entity) and inter-list moves
/// (between different entities). Uses typed function pointers for zero-erasure.
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
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone, Debug)]
/// struct Vehicle { id: usize, visits: Vec<i32> }
///
/// #[derive(Clone, Debug)]
/// struct Solution { vehicles: Vec<Vehicle>, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// fn list_len(s: &Solution, entity_idx: usize) -> usize {
///     s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
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
///     list_len, list_remove, list_insert,
///     "visits", 0,
/// );
/// ```
pub struct ListChangeMove<S, V> {
    /// Source entity index (which entity's list to remove from)
    source_entity_index: usize,
    /// Position in source list to remove from
    source_position: usize,
    /// Destination entity index (which entity's list to insert into)
    dest_entity_index: usize,
    /// Position in destination list to insert at
    dest_position: usize,
    /// Get list length for an entity
    list_len: fn(&S, usize) -> usize,
    /// Remove element at position, returns removed value
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    /// Insert element at position
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
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
    /// Creates a new list change move with typed function pointers.
    ///
    /// # Arguments
    /// * `source_entity_index` - Entity index to remove from
    /// * `source_position` - Position in source list
    /// * `dest_entity_index` - Entity index to insert into
    /// * `dest_position` - Position in destination list
    /// * `list_len` - Function to get list length
    /// * `list_remove` - Function to remove element at position
    /// * `list_insert` - Function to insert element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_index: usize,
        source_position: usize,
        dest_entity_index: usize,
        dest_position: usize,
        list_len: fn(&S, usize) -> usize,
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
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            indices: [source_entity_index, dest_entity_index],
        }
    }

    /// Returns the source entity index.
    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    /// Returns the source position.
    pub fn source_position(&self) -> usize {
        self.source_position
    }

    /// Returns the destination entity index.
    pub fn dest_entity_index(&self) -> usize {
        self.dest_entity_index
    }

    /// Returns the destination position.
    pub fn dest_position(&self) -> usize {
        self.dest_position
    }

    /// Returns true if this is an intra-list move (same entity).
    pub fn is_intra_list(&self) -> bool {
        self.source_entity_index == self.dest_entity_index
    }
}

impl<S, V> Move<S> for ListChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
        let solution = score_director.working_solution();

        // Check source position is valid
        let source_len = (self.list_len)(solution, self.source_entity_index);
        if self.source_position >= source_len {
            return false;
        }

        // Check destination position is valid
        // For intra-list, dest can be 0..=len-1 (after removal)
        // For inter-list, dest can be 0..=len
        let dest_len = (self.list_len)(solution, self.dest_entity_index);
        let max_dest = if self.is_intra_list() {
            source_len - 1 // After removal, list is shorter
        } else {
            dest_len
        };

        if self.dest_position > max_dest {
            return false;
        }

        // For intra-list moves, check for no-ops
        // Moving to same position is obviously a no-op
        // Moving forward by 1 position is also a no-op due to index adjustment:
        //   remove at source, adjusted_dest = dest-1 = source, insert at source → same list
        if self.is_intra_list() {
            if self.source_position == self.dest_position {
                return false;
            }
            // Forward move by exactly 1 is a no-op
            if self.dest_position == self.source_position + 1 {
                return false;
            }
        }

        true
    }

    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D) {
        // Notify before changes
        score_director.before_variable_changed(
            self.descriptor_index,
            self.source_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.before_variable_changed(
                self.descriptor_index,
                self.dest_entity_index,
                self.variable_name,
            );
        }

        // Remove from source
        let value = (self.list_remove)(
            score_director.working_solution_mut(),
            self.source_entity_index,
            self.source_position,
        )
        .expect("source position should be valid");

        // For intra-list moves, adjust dest position if it was after source
        let adjusted_dest = if self.is_intra_list() && self.dest_position > self.source_position {
            self.dest_position - 1
        } else {
            self.dest_position
        };

        // Insert at destination
        (self.list_insert)(
            score_director.working_solution_mut(),
            self.dest_entity_index,
            adjusted_dest,
            value.clone(),
        );

        // Notify after changes
        score_director.after_variable_changed(
            self.descriptor_index,
            self.source_entity_index,
            self.variable_name,
        );
        if !self.is_intra_list() {
            score_director.after_variable_changed(
                self.descriptor_index,
                self.dest_entity_index,
                self.variable_name,
            );
        }

        // Register undo - reverse the operation
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
        let src_entity = self.source_entity_index;
        let src_pos = self.source_position;
        let dest_entity = self.dest_entity_index;

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Remove from where we inserted
            let removed = list_remove(s, dest_entity, adjusted_dest).unwrap();
            // Insert back at original source position
            list_insert(s, src_entity, src_pos, removed);
        }));
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
}

#[cfg(test)]
#[path = "list_change_tests.rs"]
mod tests;
