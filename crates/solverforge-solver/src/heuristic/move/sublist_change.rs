//! SubListChangeMove - relocates a contiguous sublist within or between list variables.
//!
//! This move removes a range of elements from one position and inserts them at another.
//! Essential for vehicle routing where multiple consecutive stops need relocation.
//!
//! # Zero-Erasure Design
//!
//! Uses typed function pointers for list operations. No `dyn Any`, no downcasting.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Move;

/// A move that relocates a contiguous sublist from one position to another.
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
/// use solverforge_solver::heuristic::r#move::SubListChangeMove;
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
/// let m = SubListChangeMove::<Solution, i32>::new(
///     0, 1, 3,  // source: entity 0, range [1, 3)
///     1, 0,     // dest: entity 1, position 0
///     list_len, sublist_remove, sublist_insert,
///     "visits", 0,
/// );
/// ```
pub struct SubListChangeMove<S, V> {
    /// Source entity index
    source_entity_index: usize,
    /// Start of range in source list (inclusive)
    source_start: usize,
    /// End of range in source list (exclusive)
    source_end: usize,
    /// Destination entity index
    dest_entity_index: usize,
    /// Position in destination list to insert at
    dest_position: usize,
    /// Get list length for an entity
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist [start, end), returns removed elements
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert elements at position
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
    _phantom: PhantomData<V>,
}

impl<S, V> Clone for SubListChangeMove<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, V> Copy for SubListChangeMove<S, V> {}

impl<S, V: Debug> Debug for SubListChangeMove<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListChangeMove")
            .field("source_entity", &self.source_entity_index)
            .field("source_range", &(self.source_start..self.source_end))
            .field("dest_entity", &self.dest_entity_index)
            .field("dest_position", &self.dest_position)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V> SubListChangeMove<S, V> {
    /// Creates a new sublist change move with typed function pointers.
    ///
    /// # Arguments
    /// * `source_entity_index` - Entity index to remove from
    /// * `source_start` - Start of range (inclusive)
    /// * `source_end` - End of range (exclusive)
    /// * `dest_entity_index` - Entity index to insert into
    /// * `dest_position` - Position in destination list
    /// * `list_len` - Function to get list length
    /// * `sublist_remove` - Function to remove range [start, end)
    /// * `sublist_insert` - Function to insert elements at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_index: usize,
        source_start: usize,
        source_end: usize,
        dest_entity_index: usize,
        dest_position: usize,
        list_len: fn(&S, usize) -> usize,
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
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            indices: [source_entity_index, dest_entity_index],
            _phantom: PhantomData,
        }
    }

    /// Returns the source entity index.
    pub fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    /// Returns the source range start (inclusive).
    pub fn source_start(&self) -> usize {
        self.source_start
    }

    /// Returns the source range end (exclusive).
    pub fn source_end(&self) -> usize {
        self.source_end
    }

    /// Returns the sublist length.
    pub fn sublist_len(&self) -> usize {
        self.source_end.saturating_sub(self.source_start)
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

impl<S, V> Move<S> for SubListChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool {
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

        // For intra-list, check if move would actually change anything
        if self.is_intra_list() {
            // If dest is within the source range, it's a no-op
            if self.dest_position >= self.source_start && self.dest_position <= self.source_end {
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

        // Remove sublist from source
        let elements = (self.sublist_remove)(
            score_director.working_solution_mut(),
            self.source_entity_index,
            self.source_start,
            self.source_end,
        );

        // dest_position is relative to post-removal list, no adjustment needed
        let dest_pos = self.dest_position;

        // Insert at destination
        (self.sublist_insert)(
            score_director.working_solution_mut(),
            self.dest_entity_index,
            dest_pos,
            elements.clone(),
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

        // Register undo
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;
        let src_entity = self.source_entity_index;
        let src_start = self.source_start;
        let dest_entity = self.dest_entity_index;
        let sublist_len = self.sublist_len();

        score_director.register_undo(Box::new(move |s: &mut S| {
            // Remove from where we inserted
            let removed = sublist_remove(s, dest_entity, dest_pos, dest_pos + sublist_len);
            // Insert back at original position
            sublist_insert(s, src_entity, src_start, removed);
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
#[path = "sublist_change_tests.rs"]
mod tests;
