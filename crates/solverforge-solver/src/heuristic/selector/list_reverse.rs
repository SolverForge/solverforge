//! Public static adapter for the canonical list-reverse cursor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::ListReverseMove;
use crate::heuristic::selector::list_kernel::{
    NativeReverseEmitter, ReverseCursor, STATIC_REVERSE_ENTITY_SALT,
};

use super::entity::EntitySelector;
use super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};

pub struct ListReverseMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

/// Public cursor facade over the crate-private generic kernel.
pub struct ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    inner: ReverseCursor<S, NativeReverseEmitter<S, V>>,
}

impl<S, V> ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn new(inner: ReverseCursor<S, NativeReverseEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, ListReverseMove<S, V>> for ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, ListReverseMove<S, V>>> {
        self.inner.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> ListReverseMove<S, V> {
        self.inner.take_candidate(id)
    }

    fn next_owned_candidate(&mut self) -> Option<ListReverseMove<S, V>> {
        self.inner.next_owned_candidate()
    }

    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'a> fn(MoveCandidateRef<'a, S, ListReverseMove<S, V>>) -> bool,
    ) -> Option<ListReverseMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for ListReverseMoveCursor<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Item = ListReverseMove<S, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for ListReverseMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListReverseMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListReverseMoveSelector<S, V, ES> {
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_reverse,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListReverseMove<S, V>> for ListReverseMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = ListReverseMoveCursor<S, V>
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
        let canonical_entities = self
            .entity_selector
            .iter(score_director)
            .map(|reference| reference.entity_index)
            .collect::<Vec<_>>();
        let salt = STATIC_REVERSE_ENTITY_SALT ^ self.descriptor_index as u64;
        let entities = (0..canonical_entities.len())
            .map(|offset| {
                canonical_entities[context.selection_index(offset, canonical_entities.len(), salt)]
            })
            .collect::<Vec<_>>();
        let solution = score_director.working_solution();
        let route_lens = entities
            .iter()
            .map(|&entity| (self.list_len)(solution, entity))
            .collect();
        ListReverseMoveCursor::new(ReverseCursor::new(
            NativeReverseEmitter::new(
                self.list_len,
                self.list_get,
                self.list_reverse,
                self.variable_name,
                self.descriptor_index,
            ),
            entities,
            route_lens,
            context,
            self.descriptor_index,
        ))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        self.entity_selector
            .iter(score_director)
            .map(|reference| {
                let len = (self.list_len)(solution, reference.entity_index);
                if len >= 2 {
                    len * (len - 1) / 2
                } else {
                    0
                }
            })
            .sum()
    }
}
