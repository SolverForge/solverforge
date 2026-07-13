//! Public static adapter for the canonical streamed list-change kernel.
//!
//! The selector retains its historic API and emits `ListChangeMove<S, V>`.
//! Candidate enumeration itself lives in
//! `heuristic::selector::list_kernel`, shared by static and dynamic
//! selector facades.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListChangeMove;
use crate::heuristic::selector::list_kernel::{
    ChangeCursor, NativeChangeEmitter, SelectedListOwners, STATIC_CHANGE_SALTS,
};
use crate::list_placement::selected_owner_allows;

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// A move selector that relocates elements within or between list variables.
///
/// Its move type and construction API are stable.  The adapter owns only its
/// native mutation function pointers; the shared cursor owns candidate order,
/// ownership pruning, trace-compatible selected-move transfer, and cycle
/// filtering.
pub struct ListChangeMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

/// Public cursor facade that keeps the historic type surface while hiding the
/// crate-private generic emitter/kernel carrier.
pub struct ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    inner: ChangeCursor<S, NativeChangeEmitter<S, V>>,
}

impl<S, V> ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(inner: ChangeCursor<S, NativeChangeEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, ListChangeMove<S, V>> for ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListChangeMove<S, V>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListChangeMove<S, V> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<ListChangeMove<S, V>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, ListChangeMove<S, V>>) -> bool,
    ) -> Option<ListChangeMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for ListChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListChangeMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListChangeMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListChangeMoveSelector<S, V, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn: None,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

impl<S, V, ES> MoveSelector<S, ListChangeMove<S, V>> for ListChangeMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListChangeMoveCursor<S, V>
    where
        Self: 'a;

    fn open_cursor<'a, D: Director<S>>(&'a self, score_director: &D) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, D: Director<S>>(
        &'a self,
        score_director: &D,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let mut selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        selected.apply_stream_order(
            context,
            STATIC_CHANGE_SALTS.entity ^ self.descriptor_index as u64,
        );
        let owner_restrictions = crate::list_placement::selected_owner_restrictions(
            self.element_owner_fn,
            score_director.working_solution(),
            score_director
                .entity_count(self.descriptor_index)
                .unwrap_or(0),
            &selected.entities,
            &selected.route_lens,
            self.list_get,
        );
        let owners = SelectedListOwners::from_selected_restrictions(owner_restrictions);
        ListChangeMoveCursor::new(ChangeCursor::new(
            NativeChangeEmitter::new(
                self.list_len,
                self.list_get,
                self.list_remove,
                self.list_insert,
                self.variable_name,
                self.descriptor_index,
            ),
            selected.entities,
            selected.route_lens,
            context,
            STATIC_CHANGE_SALTS,
            owners,
            self.descriptor_index,
        ))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
        let Some(owner_restrictions) = crate::list_placement::selected_owner_restrictions(
            self.element_owner_fn,
            score_director.working_solution(),
            score_director
                .entity_count(self.descriptor_index)
                .unwrap_or(0),
            &selected.entities,
            &selected.route_lens,
            self.list_get,
        ) else {
            return selected.list_change_move_capacity();
        };

        if owner_restrictions.is_fixed_to_current() {
            return selected
                .route_lens
                .iter()
                .map(|&source_len| source_len * list_change_intra_destination_count(source_len))
                .sum();
        }
        let element_owners = owner_restrictions
            .mixed()
            .expect("non-fixed owner restrictions retain their matrix");

        let mut count = 0;
        for (source_idx, (&source_entity, &source_len)) in selected
            .entities
            .iter()
            .zip(selected.route_lens.iter())
            .enumerate()
        {
            for source_position in 0..source_len {
                if selected_owner_allows(element_owners, source_idx, source_position, source_entity)
                {
                    count += list_change_intra_destination_count(source_len);
                }
                for (destination_idx, (&destination_entity, &destination_len)) in selected
                    .entities
                    .iter()
                    .zip(selected.route_lens.iter())
                    .enumerate()
                {
                    if destination_idx == source_idx {
                        continue;
                    }
                    if selected_owner_allows(
                        element_owners,
                        source_idx,
                        source_position,
                        destination_entity,
                    ) {
                        count += destination_len + 1;
                    }
                }
            }
        }
        count
    }
}

#[inline]
fn list_change_intra_destination_count(source_len: usize) -> usize {
    source_len.saturating_sub(1)
}
