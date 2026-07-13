//! Public static adapter for the canonical exhaustive K-opt cursor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS,
};
use crate::heuristic::r#move::KOptMove;
use crate::heuristic::selector::list_kernel::{KOptCursor, NativeKOptEmitter};

use super::super::entity::EntitySelector;
use super::super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::config::KOptConfig;
use super::iterators::count_cut_combinations;

/// Static K-opt selector.  It preserves its public move/cursor types while
/// common cut and reconnection enumeration is owned by the shared list kernel.
pub struct KOptMoveSelector<S, V, ES> {
    entity_selector: ES,
    config: KOptConfig,
    owned_patterns: Vec<KOptReconnection>,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct KOptMoveCursor<'a, S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    inner: KOptCursor<'a, S, NativeKOptEmitter<S, V>>,
}

impl<'a, S, V> KOptMoveCursor<'a, S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn new(inner: KOptCursor<'a, S, NativeKOptEmitter<S, V>>) -> Self {
        Self { inner }
    }
}

impl<S, V> MoveCursor<S, KOptMove<S, V>> for KOptMoveCursor<'_, S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        self.inner.next_candidate()
    }
    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, KOptMove<S, V>>> {
        self.inner.candidate(id)
    }
    fn take_candidate(&mut self, id: CandidateId) -> KOptMove<S, V> {
        self.inner.take_candidate(id)
    }
    fn next_owned_candidate(&mut self) -> Option<KOptMove<S, V>> {
        self.inner.next_owned_candidate()
    }
    fn next_owned_candidate_matching(
        &mut self,
        predicate: for<'b> fn(MoveCandidateRef<'b, S, KOptMove<S, V>>) -> bool,
    ) -> Option<KOptMove<S, V>> {
        self.inner.next_owned_candidate_matching(predicate)
    }
    fn release_candidate(&mut self, id: CandidateId) -> bool {
        self.inner.release_candidate(id)
    }
}

impl<S, V> Iterator for KOptMoveCursor<'_, S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Item = KOptMove<S, V>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, ES: Debug> Debug for KOptMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("config", &self.config)
            .field("pattern_count", &self.owned_patterns.len())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V, ES> KOptMoveSelector<S, V, ES> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        config: KOptConfig,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let owned_patterns = if config.k == 3 {
            THREE_OPT_RECONNECTIONS.to_vec()
        } else {
            enumerate_reconnections(config.k)
        };
        Self {
            entity_selector,
            config,
            owned_patterns,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, KOptMove<S, V>> for KOptMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Cursor<'a>
        = KOptMoveCursor<'a, S, V>
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
        let entity_lens = self
            .entity_selector
            .iter(score_director)
            .map(|reference| {
                let entity = reference.entity_index;
                (
                    entity,
                    (self.list_len)(score_director.working_solution(), entity),
                )
            })
            .collect();
        KOptMoveCursor::new(KOptCursor::new(
            NativeKOptEmitter::new(
                self.list_len,
                self.list_get,
                self.sublist_remove,
                self.sublist_insert,
                self.variable_name,
                self.descriptor_index,
            ),
            entity_lens,
            self.config.k,
            self.config.min_segment_len,
            &self.owned_patterns,
            context,
            self.descriptor_index,
        ))
    }

    fn size<D: Director<S>>(&self, score_director: &D) -> usize {
        self.entity_selector
            .iter(score_director)
            .map(|reference| {
                let len =
                    (self.list_len)(score_director.working_solution(), reference.entity_index);
                count_cut_combinations(self.config.k, len, self.config.min_segment_len)
                    * self.owned_patterns.len()
            })
            .sum()
    }
}
