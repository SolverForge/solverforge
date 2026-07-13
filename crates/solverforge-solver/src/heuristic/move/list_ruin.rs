//! Public static facade for canonical list ruin-and-recreate mechanics.

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    merged_ruin_sources, ruin_count, ruin_do_move, ruin_entity_indices, ruin_is_doable,
    ruin_tabu_signature, ruin_undo_move, single_ruin_source, RuinSources, RuinUndo,
    RuinValueTransfer, StaticListRuinAccess,
};
use super::{Move, MoveTabuSignature};

/// A self-contained list ruin-and-recreate move.
///
/// Its public function-pointer constructor remains unchanged. Candidate
/// mutation, restore, undo, owner policy, precedence filtering, and tabu
/// mechanics live in `move::list_kernel` so runtime list moves reuse the same
/// implementation instead of translating through this facade.
pub struct ListRuinMove<S, V> {
    entity_index: usize,
    element_indices: SmallVec<[usize; 8]>,
    sources: RuinSources,
    entity_indices: SmallVec<[usize; 8]>,
    access: StaticListRuinAccess<S, V>,
    skip_empty_destinations: bool,
}

impl<S, V> Clone for ListRuinMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            entity_index: self.entity_index,
            element_indices: self.element_indices.clone(),
            sources: self.sources.clone(),
            entity_indices: self.entity_indices.clone(),
            access: self.access,
            skip_empty_destinations: self.skip_empty_destinations,
        }
    }
}

impl<S, V: Debug> Debug for ListRuinMove<S, V> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ListRuinMove")
            .field("sources", &self.sources)
            .field("variable_name", &self.access.variable_name)
            .finish()
    }
}

impl<S, V> ListRuinMove<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_index: usize,
        element_indices: &[usize],
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self::from_sources(
            single_ruin_source(entity_index, element_indices),
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_multi_source(
        sources: &[(usize, SmallVec<[usize; 8]>)],
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self::from_sources(
            merged_ruin_sources(sources),
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_sources(
        sources: RuinSources,
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let (entity_index, element_indices) = sources
            .first()
            .cloned()
            .unwrap_or_else(|| (0, SmallVec::new()));
        let entity_indices = ruin_entity_indices(&sources);
        Self {
            entity_index,
            element_indices,
            sources,
            entity_indices,
            access: StaticListRuinAccess {
                entity_count,
                list_len,
                list_get,
                list_remove,
                list_insert,
                element_owner_fn: None,
                precedence_element_count: None,
                precedence_index_to_element: None,
                precedence_successors_fn: None,
                variable_name,
                descriptor_index,
            },
            skip_empty_destinations: false,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.access.element_owner_fn = element_owner_fn;
        self
    }

    pub(crate) fn with_precedence_hooks(
        mut self,
        element_count: Option<fn(&S) -> usize>,
        index_to_element: Option<fn(&S, usize) -> V>,
        successors_fn: Option<fn(&S, V, &mut Vec<V>)>,
    ) -> Self {
        self.access.precedence_element_count = element_count;
        self.access.precedence_index_to_element = index_to_element;
        self.access.precedence_successors_fn = successors_fn;
        self
    }

    pub fn with_skip_empty_destinations(mut self, skip_empty_destinations: bool) -> Self {
        self.skip_empty_destinations = skip_empty_destinations;
        self
    }

    pub fn entity_index(&self) -> usize {
        self.entity_index
    }

    pub fn element_indices(&self) -> &[usize] {
        &self.element_indices
    }

    pub fn ruin_count(&self) -> usize {
        ruin_count(&self.sources)
    }
}

impl<S, V> Move<S> for ListRuinMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = RuinUndo;

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        ruin_is_doable(&self.access, &self.sources, score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        ruin_do_move(
            &self.access,
            &self.sources,
            self.skip_empty_destinations,
            RuinValueTransfer::CloneBeforeInsert,
            score_director,
        )
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, undo: Self::Undo) {
        ruin_undo_move(&self.access, &self.sources, undo, score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.access.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.entity_indices
    }

    fn variable_name(&self) -> &str {
        self.access.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "list_ruin"
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        ruin_tabu_signature(
            &self.access,
            &self.sources,
            &self.entity_indices,
            score_director,
        )
    }
}

#[cfg(test)]
pub(crate) fn final_positions_after_insertions(
    placements: &SmallVec<[(usize, usize); 8]>,
) -> SmallVec<[usize; 8]> {
    super::list_kernel::final_positions_after_insertions(placements)
}
