//! Public static adapter for the canonical nearby K-opt cursor.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS,
};
use crate::heuristic::r#move::KOptMove;
use crate::heuristic::selector::list_kernel::{NativeKOptEmitter, NearbyKOptCursor};

use super::super::entity::EntitySelector;
use super::super::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveSelector, MoveStreamContext,
};
use super::config::KOptConfig;
use super::distance_meter::ListPositionDistanceMeter;

pub struct NearbyKOptMoveSelector<S, V, D: ListPositionDistanceMeter<S>, ES> {
    entity_selector: ES,
    distance_meter: D,
    max_nearby: usize,
    config: KOptConfig,
    patterns: Vec<KOptReconnection>,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

pub struct NearbyKOptMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
{
    inner: NearbyKOptCursor<'a, S, NativeKOptEmitter<S, V>, &'a D>,
}

impl<'a, S, V, D> NearbyKOptMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
{
    fn new(inner: NearbyKOptCursor<'a, S, NativeKOptEmitter<S, V>, &'a D>) -> Self {
        Self { inner }
    }
}

impl<S, V, D> MoveCursor<S, KOptMove<S, V>> for NearbyKOptMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
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

impl<S, V, D> Iterator for NearbyKOptMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
{
    type Item = KOptMove<S, V>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_owned_candidate()
    }
}

impl<S, V: Debug, D: ListPositionDistanceMeter<S>, ES: Debug> Debug
    for NearbyKOptMoveSelector<S, V, D, ES>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyKOptMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("max_nearby", &self.max_nearby)
            .field("config", &self.config)
            .field("pattern_count", &self.patterns.len())
            .finish()
    }
}

impl<S: PlanningSolution, V, D: ListPositionDistanceMeter<S>, ES>
    NearbyKOptMoveSelector<S, V, D, ES>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        config: KOptConfig,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        let patterns = if config.k == 3 {
            THREE_OPT_RECONNECTIONS.to_vec()
        } else {
            enumerate_reconnections(config.k)
        };
        Self {
            entity_selector,
            distance_meter,
            max_nearby,
            config,
            patterns,
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

impl<S, V, DM, ES> MoveSelector<S, KOptMove<S, V>> for NearbyKOptMoveSelector<S, V, DM, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    DM: ListPositionDistanceMeter<S> + 'static,
    ES: EntitySelector<S>,
{
    type Cursor<'a>
        = NearbyKOptMoveCursor<'a, S, V, DM>
    where
        Self: 'a;

    fn open_cursor<'a, SD: Director<S>>(&'a self, score_director: &SD) -> Self::Cursor<'a> {
        self.open_cursor_with_context(score_director, MoveStreamContext::default())
    }

    fn open_cursor_with_context<'a, SD: Director<S>>(
        &'a self,
        score_director: &SD,
        context: MoveStreamContext,
    ) -> Self::Cursor<'a> {
        let solution = score_director.working_solution();
        let entities = self
            .entity_selector
            .iter(score_director)
            .map(|reference| reference.entity_index)
            .collect();
        NearbyKOptMoveCursor::new(NearbyKOptCursor::new(
            NativeKOptEmitter::new(
                self.list_len,
                self.list_get,
                self.sublist_remove,
                self.sublist_insert,
                self.variable_name,
                self.descriptor_index,
            ),
            solution.clone(),
            &self.distance_meter,
            entities,
            self.config.k,
            self.config.min_segment_len,
            self.max_nearby,
            &self.patterns,
            self.list_len,
            context,
            self.descriptor_index,
        ))
    }

    fn size<SD: Director<S>>(&self, score_director: &SD) -> usize {
        self.entity_selector
            .iter(score_director)
            .map(|reference| {
                let len =
                    (self.list_len)(score_director.working_solution(), reference.entity_index);
                if len < (self.config.k + 1) * self.config.min_segment_len {
                    0
                } else {
                    len.saturating_sub(self.config.k)
                        * self.max_nearby.pow((self.config.k - 1) as u32)
                        * self.patterns.len()
                }
            })
            .sum()
    }
}
