//! SublistChangeMove - relocates a contiguous sublist within or between list variables.
//!
//! This move removes a range of elements from one position and inserts them at another.
//! Essential for vehicle routing where multiple consecutive stops need relocation.
//!
//! # Zero-Erasure Design
//!
//! Uses concrete function pointers for list operations. No `dyn Any`, no downcasting.

use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    sublist_change_candidate_trace_identity, sublist_change_do_move, sublist_change_is_doable,
    sublist_change_tabu_signature, sublist_change_undo_move, StaticListWindowAccess,
};
use super::segment_layout::SegmentRelocationCoords;
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
pub(crate) struct SublistChangeMove<S, V> {
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
    /// Creates a new sublist change move with concrete function pointers.
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
    pub(crate) fn new(
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

    pub(crate) fn source_entity_index(&self) -> usize {
        self.source_entity_index
    }

    pub(crate) fn source_start(&self) -> usize {
        self.source_start
    }

    pub(crate) fn source_end(&self) -> usize {
        self.source_end
    }

    pub(crate) fn dest_entity_index(&self) -> usize {
        self.dest_entity_index
    }

    pub(crate) fn dest_position(&self) -> usize {
        self.dest_position
    }

    pub(crate) fn is_intra_list(&self) -> bool {
        self.source_entity_index == self.dest_entity_index
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

    fn coordinates(&self) -> SegmentRelocationCoords {
        SegmentRelocationCoords::new(
            self.source_entity_index,
            self.source_start,
            self.source_end,
            self.dest_entity_index,
            self.dest_position,
        )
    }
}

impl<S, V> Move<S> for SublistChangeMove<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        sublist_change_is_doable(&self.access(), self.coordinates(), score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        sublist_change_do_move(&self.access(), self.coordinates(), score_director);
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        sublist_change_undo_move(&self.access(), self.coordinates(), score_director);
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
        "sublist_change"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> Cow<'_, MoveTabuSignature> {
        Cow::Owned(sublist_change_tabu_signature(
            &self.access(),
            self.coordinates(),
            score_director,
        ))
    }

    fn candidate_trace_identity(&self) -> Option<crate::stats::CandidateTraceIdentity> {
        Some(sublist_change_candidate_trace_identity(
            &self.access(),
            self.coordinates(),
        ))
    }
}
