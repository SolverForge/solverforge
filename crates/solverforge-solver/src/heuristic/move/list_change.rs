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
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::{ScoreDirector, ShadowVariableSupport};

use super::traits::Move;

/// A move that relocates an element from one list position to another.
///
/// Supports both intra-list moves (within same entity) and inter-list moves
/// (between different entities). Uses typed function pointers for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `V` - The list element value type
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
    /// Get element index at position (for O(1) shadow updates)
    list_get_element_idx: fn(&S, usize, usize) -> usize,
    variable_name: &'static str,
    descriptor_index: usize,
    /// Store indices for entity_indices()
    indices: [usize; 2],
}

impl<S, V> Clone for ListChangeMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            source_entity_index: self.source_entity_index,
            source_position: self.source_position,
            dest_entity_index: self.dest_entity_index,
            dest_position: self.dest_position,
            list_len: self.list_len,
            list_remove: self.list_remove,
            list_insert: self.list_insert,
            list_get_element_idx: self.list_get_element_idx,
            variable_name: self.variable_name,
            descriptor_index: self.descriptor_index,
            indices: self.indices,
        }
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_index: usize,
        source_position: usize,
        dest_entity_index: usize,
        dest_position: usize,
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_get_element_idx: fn(&S, usize, usize) -> usize,
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
            list_get_element_idx,
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

    /// Checks if this move is doable given the solution state.
    fn is_doable_impl<C>(&self, score_director: &ScoreDirector<S, C>) -> bool
    where
        S: PlanningSolution + ShadowVariableSupport,
        S::Score: Score,
        C: ConstraintSet<S, S::Score>,
    {
        let solution = score_director.working_solution();

        // Check source position is valid
        let source_len = (self.list_len)(solution, self.source_entity_index);
        if self.source_position >= source_len {
            return false;
        }

        // Check destination position is valid
        let dest_len = (self.list_len)(solution, self.dest_entity_index);
        let max_dest = if self.is_intra_list() {
            source_len - 1
        } else {
            dest_len
        };

        if self.dest_position > max_dest {
            return false;
        }

        // For intra-list moves, check for no-ops
        if self.is_intra_list() {
            if self.source_position == self.dest_position {
                return false;
            }
            if self.dest_position == self.source_position + 1 {
                return false;
            }
        }

        true
    }

    /// Executes this move on the score director.
    fn do_move_impl<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        S: PlanningSolution + ShadowVariableSupport,
        S::Score: Score,
        C: ConstraintSet<S, S::Score>,
        V: Clone,
    {
        let element_idx = (self.list_get_element_idx)(
            score_director.working_solution(),
            self.source_entity_index,
            self.source_position,
        );

        let adjusted_dest = if self.is_intra_list() && self.dest_position > self.source_position {
            self.dest_position - 1
        } else {
            self.dest_position
        };

        // Before removal
        score_director.before_list_element_changed(
            self.source_entity_index,
            self.source_position,
            element_idx,
        );

        // Remove
        let value = (self.list_remove)(
            score_director.working_solution_mut(),
            self.source_entity_index,
            self.source_position,
        )
        .expect("source position should be valid");

        // After removal
        score_director.after_list_element_changed(
            self.source_entity_index,
            self.source_position,
            element_idx,
        );

        // Before insertion
        score_director.before_list_element_changed(
            self.dest_entity_index,
            adjusted_dest,
            element_idx,
        );

        // Insert
        (self.list_insert)(
            score_director.working_solution_mut(),
            self.dest_entity_index,
            adjusted_dest,
            value,
        );

        // After insertion
        score_director.after_list_element_changed(
            self.dest_entity_index,
            adjusted_dest,
            element_idx,
        );
    }
}

impl<S, V> Move<S> for ListChangeMove<S, V>
where
    S: PlanningSolution + ShadowVariableSupport,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn is_doable<C>(&self, score_director: &ScoreDirector<S, C>) -> bool
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.is_doable_impl(score_director)
    }

    fn do_move<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        C: ConstraintSet<S, S::Score>,
    {
        self.do_move_impl(score_director);
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
