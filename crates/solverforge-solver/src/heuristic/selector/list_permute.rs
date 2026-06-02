/* List permute move selector for contiguous intra-list window permutations. */

use std::fmt::Debug;
use std::marker::PhantomData;

use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::{ListPermuteMove, MAX_LIST_PERMUTE_WINDOW_SIZE};
use crate::list_placement::{selected_segment_allows, OwnerRestriction, SelectedOwnerRestrictions};

use super::entity::EntitySelector;
use super::list_support::{collect_selected_entities, ordered_index};
use super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::precedence_route::{PrecedenceRouteGraph, PrecedenceRouteHooks};

pub struct ListPermuteMoveSelector<S, V, ES> {
    entity_selector: ES,
    min_window_size: usize,
    max_window_size: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    precedence_route_hooks: Option<PrecedenceRouteHooks<S, V>>,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    store: CandidateStore<S, ListPermuteMove<S, V>>,
    entities: Vec<usize>,
    route_lens: Vec<usize>,
    context: MoveStreamContext,
    element_owners: Option<Vec<Vec<OwnerRestriction>>>,
    fixed_to_current_entity: bool,
    precedence_route_graph: Option<PrecedenceRouteGraph>,
    entity_idx: usize,
    start_offset: usize,
    size_offset: usize,
    permutation_offset: usize,
    current_window: Option<(usize, usize)>,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    min_window_size: usize,
    max_window_size: usize,
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<S, V> ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        entities: Vec<usize>,
        route_lens: Vec<usize>,
        context: MoveStreamContext,
        element_owners: Option<Vec<Vec<OwnerRestriction>>>,
        fixed_to_current_entity: bool,
        precedence_route_graph: Option<PrecedenceRouteGraph>,
        min_window_size: usize,
        max_window_size: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            entities,
            route_lens,
            context,
            element_owners,
            fixed_to_current_entity,
            precedence_route_graph,
            entity_idx: 0,
            start_offset: 0,
            size_offset: 0,
            permutation_offset: 0,
            current_window: None,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            min_window_size,
            max_window_size,
            variable_name,
            descriptor_index,
        }
    }

    pub(crate) fn with_precedence_route_graph(
        mut self,
        precedence_route_graph: Option<PrecedenceRouteGraph>,
    ) -> Self {
        self.precedence_route_graph = precedence_route_graph;
        self
    }

    fn advance_entity(&mut self) {
        self.entity_idx += 1;
        self.start_offset = 0;
        self.size_offset = 0;
        self.permutation_offset = 0;
        self.current_window = None;
    }

    fn advance_start(&mut self) {
        self.start_offset += 1;
        self.size_offset = 0;
        self.permutation_offset = 0;
        self.current_window = None;
    }

    fn window_owner_allows(&self, start: usize, end: usize) -> bool {
        if self.fixed_to_current_entity {
            return true;
        }
        let Some(element_owners) = &self.element_owners else {
            return true;
        };
        selected_segment_allows(
            element_owners,
            self.entity_idx,
            start,
            end,
            self.entities[self.entity_idx],
        )
    }

    fn push_move(
        &mut self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> CandidateId {
        self.store.push(ListPermuteMove::new(
            entity,
            start,
            end,
            permutation,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }
}

impl<S, V> MoveCursor<S, ListPermuteMove<S, V>> for ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if self.entity_idx >= self.entities.len() {
                return None;
            }
            let entity = self.entities[self.entity_idx];
            let route_len = self.route_lens[self.entity_idx];
            if route_len < self.min_window_size {
                self.advance_entity();
                continue;
            }

            if let Some((start, size)) = self.current_window {
                let permutation_count = factorial(size).saturating_sub(1);
                if self.permutation_offset < permutation_count {
                    let rank = ordered_index(
                        self.permutation_offset,
                        permutation_count,
                        self.context,
                        0x91D7_9E8A_0000_0004
                            ^ entity as u64
                            ^ start as u64
                            ^ size as u64
                            ^ self.descriptor_index as u64,
                    ) + 1;
                    self.permutation_offset += 1;
                    let permutation = nth_permutation(size, rank);
                    if self.precedence_route_graph.as_ref().is_some_and(|graph| {
                        graph.intra_list_permutation_introduces_cycle(
                            entity,
                            start,
                            start + size,
                            &permutation,
                        )
                    }) {
                        continue;
                    }
                    return Some(self.push_move(entity, start, start + size, permutation));
                }
                self.current_window = None;
                self.size_offset += 1;
                self.permutation_offset = 0;
            }

            if self.start_offset >= route_len {
                self.advance_entity();
                continue;
            }

            let start = ordered_index(
                self.start_offset,
                route_len,
                self.context,
                0x91D7_9E8A_0000_0002 ^ entity as u64 ^ self.descriptor_index as u64,
            );
            let max_valid = self.max_window_size.min(route_len - start);
            if max_valid < self.min_window_size {
                self.advance_start();
                continue;
            }
            let size_count = max_valid - self.min_window_size + 1;
            if self.size_offset >= size_count {
                self.advance_start();
                continue;
            }
            let size = self.min_window_size
                + ordered_index(
                    self.size_offset,
                    size_count,
                    self.context,
                    0x91D7_9E8A_0000_0003 ^ entity as u64 ^ start as u64,
                );
            if !self.window_owner_allows(start, start + size) {
                self.size_offset += 1;
                continue;
            }
            self.current_window = Some((start, size));
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListPermuteMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListPermuteMove<S, V> {
        self.store.take_candidate(id)
    }
}

impl<S, V> Iterator for ListPermuteMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Item = ListPermuteMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
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
            precedence_route_hooks: None,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    pub fn with_element_owner_fn(mut self, owner_fn: Option<fn(&S, &V) -> Option<usize>>) -> Self {
        self.element_owner_fn = owner_fn;
        self
    }

    pub(crate) fn with_precedence_route_hooks(
        mut self,
        precedence_route_hooks: Option<PrecedenceRouteHooks<S, V>>,
    ) -> Self {
        self.precedence_route_hooks = precedence_route_hooks;
        self
    }

    pub(crate) fn precedence_route_hooks(&self) -> Option<PrecedenceRouteHooks<S, V>> {
        self.precedence_route_hooks
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
        let fixed_to_current_entity = owner_restrictions
            .as_ref()
            .is_some_and(SelectedOwnerRestrictions::is_fixed_to_current);
        let element_owners = owner_restrictions.and_then(SelectedOwnerRestrictions::into_mixed);

        ListPermuteMoveCursor::new(
            selected.entities,
            selected.route_lens,
            context,
            element_owners,
            fixed_to_current_entity,
            None,
            self.min_window_size,
            self.max_window_size,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
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
        let element_owners = owner_restrictions
            .mixed()
            .expect("non-fixed owner restrictions must retain mixed owner matrix");

        let mut count = 0;
        for (entity_idx, &route_len) in selected.route_lens.iter().enumerate() {
            for start in 0..route_len {
                let max_valid = self.max_window_size.min(route_len - start);
                for size in self.min_window_size..=max_valid {
                    if selected_segment_allows(
                        element_owners,
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

fn count_list_permute_moves_for_len(
    route_len: usize,
    min_window_size: usize,
    max_window_size: usize,
) -> usize {
    if route_len < min_window_size {
        return 0;
    }
    let mut count = 0;
    for start in 0..route_len {
        let max_valid = max_window_size.min(route_len - start);
        for size in min_window_size..=max_valid {
            count += factorial(size).saturating_sub(1);
        }
    }
    count
}

fn factorial(value: usize) -> usize {
    (2..=value).product()
}

fn nth_permutation(len: usize, mut rank: usize) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut remaining: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> = (0..len).collect();
    let mut permutation = smallvec![];
    for pos in 0..len {
        let suffix = len - pos - 1;
        let step = factorial(suffix);
        let index = if step == 0 { 0 } else { rank / step };
        rank %= step.max(1);
        permutation.push(remaining.remove(index));
    }
    permutation
}
