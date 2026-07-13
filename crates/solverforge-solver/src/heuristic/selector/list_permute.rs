//! Public static adapter for the canonical streamed list-permute cursor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ListPermuteMove, MAX_LIST_PERMUTE_WINDOW_SIZE};
use crate::heuristic::selector::list_kernel::{
    count_list_permute_moves_for_len, factorial, NativeWindowEmitter, PermuteCursor,
    SelectedListOwners,
};
use crate::list_placement::selected_segment_allows;

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

pub struct ListPermuteMoveSelector<S, V, ES> {
    entity_selector: ES,
    min_window_size: usize,
    max_window_size: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    inner: PermuteCursor<S, NativeWindowEmitter<S, V>>,
}

impl<S, V> ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn new(inner: PermuteCursor<S, NativeWindowEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, ListPermuteMove<S, V>> for ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }
    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListPermuteMove<S, V>>> {
        self.inner.candidate(id)
    }
    fn take_candidate(&mut self, id: CandidateId) -> ListPermuteMove<S, V> {
        self.inner.take_candidate(id)
    }
    fn next_owned_candidate(&mut self) -> Option<ListPermuteMove<S, V>> {
        self.inner.next_owned_candidate()
    }
    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, ListPermuteMove<S, V>>) -> bool,
    ) -> Option<ListPermuteMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }
    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Item = ListPermuteMove<S, V>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListPermuteMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListPermuteMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_window_size", &self.min_window_size)
            .field("max_window_size", &self.max_window_size)
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S, V, ES> ListPermuteMoveSelector<S, V, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        min_window_size: usize,
        max_window_size: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(
            min_window_size >= 2,
            "list permute min_window_size must be at least 2",
        );
        assert!(
            min_window_size <= max_window_size,
            "list permute min_window_size must be <= max_window_size",
        );
        assert!(
            max_window_size <= MAX_LIST_PERMUTE_WINDOW_SIZE,
            "list permute max_window_size must be <= {MAX_LIST_PERMUTE_WINDOW_SIZE}",
        );
        Self {
            entity_selector,
            min_window_size,
            max_window_size,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            element_owner_fn: None,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(mut self, owner_fn: Option<fn(&S, &V) -> Option<usize>>) -> Self {
        self.element_owner_fn = owner_fn;
        self
    }
}

impl<S, V, ES> MoveSelector<S, ListPermuteMove<S, V>> for ListPermuteMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListPermuteMoveCursor<S, V>
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
        let selected =
            collect_selected_entities(&self.entity_selector, score_director, self.list_len);
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
        ListPermuteMoveCursor::new(PermuteCursor::new(
            NativeWindowEmitter::new(
                self.list_len,
                self.list_get,
                self.sublist_remove,
                self.sublist_insert,
                self.variable_name,
                self.descriptor_index,
            ),
            selected.entities,
            selected.route_lens,
            context,
            self.min_window_size,
            self.max_window_size,
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
            return selected
                .route_lens
                .iter()
                .map(|&route_len| {
                    count_list_permute_moves_for_len(
                        route_len,
                        self.min_window_size,
                        self.max_window_size,
                    )
                })
                .sum();
        };
        if owner_restrictions.is_fixed_to_current() {
            return selected
                .route_lens
                .iter()
                .map(|&route_len| {
                    count_list_permute_moves_for_len(
                        route_len,
                        self.min_window_size,
                        self.max_window_size,
                    )
                })
                .sum();
        }
        let owners = owner_restrictions
            .mixed()
            .expect("non-fixed owner restrictions retain their matrix");
        let mut count = 0;
        for (entity_idx, &route_len) in selected.route_lens.iter().enumerate() {
            for start in 0..route_len {
                let max_valid = self.max_window_size.min(route_len - start);
                for size in self.min_window_size..=max_valid {
                    if selected_segment_allows(
                        owners,
                        entity_idx,
                        start,
                        start + size,
                        selected.entities[entity_idx],
                    ) {
                        count += factorial(size).saturating_sub(1);
                    }
                }
            }
        }
        count
    }
}
