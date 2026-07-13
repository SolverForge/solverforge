//! Public static adapter for the canonical streamed sublist-swap cursor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::SublistSwapMove;
use crate::list_placement::selected_segment_allows;

use super::list_kernel::{
    NativeWindowEmitter, SelectedListOwners, SublistSwapCursor, STATIC_SUBLIST_SWAP_ENTITY_SALT,
};

use super::entity::EntitySelector;
use super::list_support::collect_selected_entities;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::sublist_support::{count_intra_sublist_swap_moves_for_len, count_sublist_segments};

pub struct SublistSwapMoveSelector<S, V, ES> {
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

pub struct SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    inner: SublistSwapCursor<S, NativeWindowEmitter<S, V>>,
}

impl<S, V> SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn new(inner: SublistSwapCursor<S, NativeWindowEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, SublistSwapMove<S, V>> for SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }
    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, SublistSwapMove<S, V>>> {
        self.inner.candidate(id)
    }
    fn take_candidate(&mut self, id: CandidateId) -> SublistSwapMove<S, V> {
        self.inner.take_candidate(id)
    }
    fn next_owned_candidate(&mut self) -> Option<SublistSwapMove<S, V>> {
        self.inner.next_owned_candidate()
    }
    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, SublistSwapMove<S, V>>) -> bool,
    ) -> Option<SublistSwapMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }
    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for SublistSwapMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Item = SublistSwapMove<S, V>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for SublistSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SublistSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SublistSwapMoveSelector<S, V, ES> {
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

impl<S, V, ES> MoveSelector<S, SublistSwapMove<S, V>> for SublistSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = SublistSwapMoveCursor<S, V>
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
            STATIC_SUBLIST_SWAP_ENTITY_SALT ^ self.descriptor_index as u64,
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
        SublistSwapMoveCursor::new(SublistSwapCursor::new(
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
            return unfiltered_sublist_swap_size(
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
                    count_intra_sublist_swap_moves_for_len(
                        route_len,
                        self.min_sublist_size,
                        self.max_sublist_size,
                    )
                })
                .sum();
        }
        let owners = owner_restrictions
            .mixed()
            .expect("non-fixed owner restrictions retain their matrix");
        let mut count = 0;
        for (first_idx, &first_entity) in selected.entities.iter().enumerate() {
            let first_len = selected.route_lens[first_idx];
            for first_start in 0..first_len {
                let first_max = self.max_sublist_size.min(first_len - first_start);
                for first_size in self.min_sublist_size..=first_max {
                    let first_end = first_start + first_size;
                    for (second_idx, &second_entity) in
                        selected.entities.iter().enumerate().skip(first_idx)
                    {
                        let second_len = selected.route_lens[second_idx];
                        for second_start in 0..second_len {
                            let second_max = self.max_sublist_size.min(second_len - second_start);
                            for second_size in self.min_sublist_size..=second_max {
                                let second_end = second_start + second_size;
                                if first_idx == second_idx
                                    && (second_start < first_end
                                        || (first_start == second_start && first_end == second_end))
                                {
                                    continue;
                                }
                                let allowed = if first_entity == second_entity {
                                    selected_segment_allows(
                                        owners,
                                        first_idx,
                                        first_start,
                                        first_end,
                                        first_entity,
                                    ) && selected_segment_allows(
                                        owners,
                                        second_idx,
                                        second_start,
                                        second_end,
                                        first_entity,
                                    )
                                } else {
                                    selected_segment_allows(
                                        owners,
                                        first_idx,
                                        first_start,
                                        first_end,
                                        second_entity,
                                    ) && selected_segment_allows(
                                        owners,
                                        second_idx,
                                        second_start,
                                        second_end,
                                        first_entity,
                                    )
                                };
                                if allowed {
                                    count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        count
    }
}

fn unfiltered_sublist_swap_size(
    route_lens: &[usize],
    min_sublist_size: usize,
    max_sublist_size: usize,
) -> usize {
    let segment_counts = route_lens
        .iter()
        .map(|&route_len| count_sublist_segments(route_len, min_sublist_size, max_sublist_size))
        .collect::<Vec<_>>();
    let intra = route_lens
        .iter()
        .map(|&route_len| {
            count_intra_sublist_swap_moves_for_len(route_len, min_sublist_size, max_sublist_size)
        })
        .sum::<usize>();
    let inter = (0..route_lens.len())
        .flat_map(|left| (left + 1..route_lens.len()).map(move |right| (left, right)))
        .map(|(left, right)| segment_counts[left] * segment_counts[right])
        .sum::<usize>();
    intra + inter
}
