//! Public static adapter for the canonical streamed list-swap kernel.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListSwapMove;
use crate::heuristic::selector::list_kernel::{
    NativeSwapEmitter, SelectedListOwners, SwapCursor, STATIC_SWAP_SALTS,
};
use crate::list_placement::selected_owner_allows;

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

/// Native list swap selector.  It retains its public move type while the
/// shared cursor owns ordering, ownership pruning, and candidate ownership.
pub struct ListSwapMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

/// Public cursor facade that hides the crate-private generic emitter/kernel.
pub struct ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    inner: SwapCursor<S, NativeSwapEmitter<S, V>>,
}

impl<S, V> ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(inner: SwapCursor<S, NativeSwapEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, ListSwapMove<S, V>> for ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListSwapMove<S, V>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListSwapMove<S, V> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<ListSwapMove<S, V>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, ListSwapMove<S, V>>) -> bool,
    ) -> Option<ListSwapMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for ListSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = ListSwapMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListSwapMoveSelector<S, V, ES> {
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_set,
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

impl<S, V, ES> MoveSelector<S, ListSwapMove<S, V>> for ListSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListSwapMoveCursor<S, V>
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
            STATIC_SWAP_SALTS.entity ^ self.descriptor_index as u64,
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
        ListSwapMoveCursor::new(SwapCursor::new(
            NativeSwapEmitter::new(
                self.list_len,
                self.list_get,
                self.list_set,
                self.variable_name,
                self.descriptor_index,
            ),
            selected.entities,
            selected.route_lens,
            context,
            STATIC_SWAP_SALTS,
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
            return selected.list_swap_move_capacity();
        };

        if owner_restrictions.is_fixed_to_current() {
            return selected
                .route_lens
                .iter()
                .map(|&len| len * len.saturating_sub(1) / 2)
                .sum();
        }
        let element_owners = owner_restrictions
            .mixed()
            .expect("non-fixed owner restrictions retain their matrix");
        let mut count = 0;
        for (left_idx, (&left_entity, &left_len)) in selected
            .entities
            .iter()
            .zip(selected.route_lens.iter())
            .enumerate()
        {
            for left_position in 0..left_len {
                for right_position in left_position + 1..left_len {
                    if selected_owner_allows(element_owners, left_idx, left_position, left_entity)
                        && selected_owner_allows(
                            element_owners,
                            left_idx,
                            right_position,
                            left_entity,
                        )
                    {
                        count += 1;
                    }
                }
                for (right_idx, (&right_entity, &right_len)) in selected
                    .entities
                    .iter()
                    .zip(selected.route_lens.iter())
                    .enumerate()
                    .skip(left_idx + 1)
                {
                    for right_position in 0..right_len {
                        if selected_owner_allows(
                            element_owners,
                            left_idx,
                            left_position,
                            right_entity,
                        ) && selected_owner_allows(
                            element_owners,
                            right_idx,
                            right_position,
                            left_entity,
                        ) {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }
}
