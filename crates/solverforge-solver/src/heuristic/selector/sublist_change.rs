//! Public static adapter for the canonical streamed sublist-change cursor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::SublistChangeMove;
use crate::list_placement::selected_segment_allows;

use super::list_kernel::{
    NativeWindowEmitter, SelectedListOwners, SublistChangeCursor, STATIC_SUBLIST_CHANGE_SALTS,
};

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::sublist_support::count_sublist_change_moves_for_len;

pub struct SublistChangeMoveSelector<S, V, ES> {
    entity_selector: ES,
    min_sublist_size: usize,
    max_sublist_size: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    inner: SublistChangeCursor<S, NativeWindowEmitter<S, V>>,
}

impl<S, V> SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(inner: SublistChangeCursor<S, NativeWindowEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, SublistChangeMove<S, V>> for SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }
    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, SublistChangeMove<S, V>>> {
        self.inner.candidate(id)
    }
    fn take_candidate(&mut self, id: CandidateId) -> SublistChangeMove<S, V> {
        self.inner.take_candidate(id)
    }
    fn next_owned_candidate(&mut self) -> Option<SublistChangeMove<S, V>> {
        self.inner.next_owned_candidate()
    }
    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, SublistChangeMove<S, V>>) -> bool,
    ) -> Option<SublistChangeMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }
    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for SublistChangeMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = SublistChangeMove<S, V>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for SublistChangeMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SublistChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SublistChangeMoveSelector<S, V, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        min_sublist_size: usize,
        max_sublist_size: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        assert!(min_sublist_size >= 1, "min_sublist_size must be at least 1");
        assert!(
            max_sublist_size >= min_sublist_size,
            "max_sublist_size must be >= min_sublist_size"
        );
        Self {
            entity_selector,
            min_sublist_size,
            max_sublist_size,
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

    pub fn with_element_owner_fn(
        mut self,
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    ) -> Self {
        self.element_owner_fn = element_owner_fn;
        self
    }
}

impl<S, V, ES> MoveSelector<S, SublistChangeMove<S, V>> for SublistChangeMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = SublistChangeMoveCursor<S, V>
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
            STATIC_SUBLIST_CHANGE_SALTS.entity ^ self.descriptor_index as u64,
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
        SublistChangeMoveCursor::new(SublistChangeCursor::new(
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
            STATIC_SUBLIST_CHANGE_SALTS,
            self.min_sublist_size,
            self.max_sublist_size,
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
            return unfiltered_sublist_change_size(
                &selected.route_lens,
                self.min_sublist_size,
                self.max_sublist_size,
            );
        };
        if owner_restrictions.is_fixed_to_current() {
            return selected
                .route_lens
                .iter()
                .map(|&route_len| {
                    count_sublist_change_moves_for_len(
                        route_len,
                        0,
                        self.min_sublist_size,
                        self.max_sublist_size,
                    )
                })
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
            for segment_start in 0..source_len {
                let max_segment = self.max_sublist_size.min(source_len - segment_start);
                for segment_size in self.min_sublist_size..=max_segment {
                    let segment_end = segment_start + segment_size;
                    if selected_segment_allows(
                        element_owners,
                        source_idx,
                        segment_start,
                        segment_end,
                        source_entity,
                    ) {
                        count += source_len - segment_size;
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
                        if selected_segment_allows(
                            element_owners,
                            source_idx,
                            segment_start,
                            segment_end,
                            destination_entity,
                        ) {
                            count += destination_len + 1;
                        }
                    }
                }
            }
        }
        count
    }
}

fn unfiltered_sublist_change_size(
    route_lens: &[usize],
    min_sublist_size: usize,
    max_sublist_size: usize,
) -> usize {
    let total_elements = route_lens.iter().sum::<usize>();
    let entity_count = route_lens.len();
    route_lens
        .iter()
        .map(|&route_len| {
            let inter_destinations =
                total_elements.saturating_sub(route_len) + entity_count.saturating_sub(1);
            count_sublist_change_moves_for_len(
                route_len,
                inter_destinations,
                min_sublist_size,
                max_sublist_size,
            )
        })
        .sum()
}
