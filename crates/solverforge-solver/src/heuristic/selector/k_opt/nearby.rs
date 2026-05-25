// Nearby k-opt move selector for improved performance.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS,
};
use crate::heuristic::r#move::{CutPoint, KOptMove};

use super::super::entity::EntitySelector;
use super::super::move_selector::{
    CandidateId, CandidateStore, MoveCandidateRef, MoveCursor, MoveSelector,
};
use super::config::KOptConfig;
use super::distance_meter::ListPositionDistanceMeter;

/// A k-opt move selector with nearby selection for improved performance.
///
/// Instead of enumerating all O(n^k) cut combinations, uses distance-based
/// pruning to reduce to O(n * m^(k-1)) where m = max_nearby_size.
///
/// # How It Works
///
/// 1. First cut: all positions in the route
/// 2. Second cut: only positions nearby (by element distance) to first cut
/// 3. Third cut: only positions nearby to second cut
/// 4. etc.
///
/// This dramatically reduces the search space for large routes.
pub struct NearbyKOptMoveSelector<S, V, D: ListPositionDistanceMeter<S>, ES> {
    // Selects entities (routes) to apply k-opt to.
    entity_selector: ES,
    // Distance meter for nearby selection.
    distance_meter: D,
    // Maximum nearby positions to consider.
    max_nearby: usize,
    // K-opt configuration.
    config: KOptConfig,
    // Reconnection patterns.
    patterns: Vec<&'static KOptReconnection>,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    // Remove sublist.
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    // Insert sublist.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    // Variable name.
    variable_name: &'static str,
    // Descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
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
        let patterns: Vec<&'static KOptReconnection> = if config.k == 3 {
            THREE_OPT_RECONNECTIONS.iter().collect()
        } else {
            let generated = enumerate_reconnections(config.k);
            let leaked: &'static [KOptReconnection] = Box::leak(generated.into_boxed_slice());
            leaked.iter().collect()
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

fn nearby_positions<S, D>(
    solution: &S,
    distance_meter: &D,
    max_nearby: usize,
    entity_idx: usize,
    origin: usize,
    len: usize,
) -> Vec<usize>
where
    D: ListPositionDistanceMeter<S>,
{
    let mut positions: Vec<(usize, f64)> = (0..len)
        .filter(|&p| p != origin)
        .map(|p| {
            let dist = distance_meter.distance(solution, entity_idx, origin, p);
            (p, dist)
        })
        .collect();

    positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    positions.truncate(max_nearby);
    positions.into_iter().map(|(p, _)| p).collect()
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
        let solution = score_director.working_solution();
        let entities = self
            .entity_selector
            .iter(score_director)
            .map(|entity_ref| entity_ref.entity_index)
            .collect();
        NearbyKOptMoveCursor::new(
            solution.clone(),
            &self.distance_meter,
            entities,
            self.config.k,
            self.config.min_segment_len,
            self.max_nearby,
            &self.patterns,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }

    fn size<SD: Director<S>>(&self, score_director: &SD) -> usize {
        // Approximate size: n * m^(k-1) * patterns
        let k = self.config.k;
        let m = self.max_nearby;
        let pattern_count = self.patterns.len();

        self.entity_selector
            .iter(score_director)
            .map(|entity_ref| {
                let solution = score_director.working_solution();
                let len = (self.list_len)(solution, entity_ref.entity_index);
                if len < (k + 1) * self.config.min_segment_len {
                    0
                } else {
                    // Approximate: first cut has ~len choices, others have ~m choices
                    len.saturating_sub(k) * m.pow((k - 1) as u32) * pattern_count
                }
            })
            .sum()
    }
}

pub struct NearbyKOptMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
{
    store: CandidateStore<S, KOptMove<S, V>>,
    solution: S,
    distance_meter: &'a D,
    entities: Vec<usize>,
    entity_offset: usize,
    cut_state: Option<NearbyCutState>,
    pending_cuts: Option<Vec<CutPoint>>,
    pattern_offset: usize,
    k: usize,
    min_seg: usize,
    max_nearby: usize,
    patterns: &'a [&'static KOptReconnection],
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
}

impl<'a, S, V, D> NearbyKOptMoveCursor<'a, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
{
    #[allow(clippy::too_many_arguments)]
    fn new(
        solution: S,
        distance_meter: &'a D,
        entities: Vec<usize>,
        k: usize,
        min_seg: usize,
        max_nearby: usize,
        patterns: &'a [&'static KOptReconnection],
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            store: CandidateStore::new(),
            solution,
            distance_meter,
            entities,
            entity_offset: 0,
            cut_state: None,
            pending_cuts: None,
            pattern_offset: 0,
            k,
            min_seg,
            max_nearby,
            patterns,
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
        }
    }

    fn load_next_cut_state(&mut self) -> bool {
        while self.entity_offset < self.entities.len() {
            let entity_idx = self.entities[self.entity_offset];
            self.entity_offset += 1;
            let len = (self.list_len)(&self.solution, entity_idx);
            let state = NearbyCutState::new(entity_idx, self.k, len, self.min_seg, self.max_nearby);
            if !state.is_done() {
                self.cut_state = Some(state);
                return true;
            }
        }
        false
    }
}

impl<S, V, D> MoveCursor<S, KOptMove<S, V>> for NearbyKOptMoveCursor<'_, S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        loop {
            if let Some(cuts) = self.pending_cuts.as_ref() {
                if self.pattern_offset < self.patterns.len() {
                    let pattern = self.patterns[self.pattern_offset];
                    self.pattern_offset += 1;
                    return Some(self.store.push(KOptMove::new(
                        cuts,
                        pattern,
                        self.list_len,
                        self.list_get,
                        self.sublist_remove,
                        self.sublist_insert,
                        self.variable_name,
                        self.descriptor_index,
                    )));
                }
                self.pending_cuts = None;
                self.pattern_offset = 0;
            }

            if self.cut_state.is_none() && !self.load_next_cut_state() {
                return None;
            }

            if let Some(state) = self.cut_state.as_mut() {
                if let Some(mut cuts) = state.next_cuts(&self.solution, self.distance_meter) {
                    cuts.sort_by_key(|c| c.position());
                    self.pending_cuts = Some(cuts);
                    self.pattern_offset = 0;
                    continue;
                }
            }
            self.cut_state = None;
        }
    }

    fn candidate(&self, id: CandidateId) -> Option<MoveCandidateRef<'_, S, KOptMove<S, V>>> {
        self.store.candidate(id)
    }

    fn take_candidate(&mut self, id: CandidateId) -> KOptMove<S, V> {
        self.store.take_candidate(id)
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
        let id = self.next_candidate()?;
        Some(self.take_candidate(id))
    }
}

// Iterator state for nearby k-cut combinations.
struct NearbyCutState {
    entity_idx: usize,
    k: usize,
    len: usize,
    max_nearby: usize,
    min_seg: usize,
    // Stack of (position, nearby_iterator_index)
    stack: Vec<(usize, usize)>,
    // Nearby positions for each level
    nearby_cache: Vec<Vec<usize>>,
    done: bool,
}

impl NearbyCutState {
    fn new(entity_idx: usize, k: usize, len: usize, min_seg: usize, max_nearby: usize) -> Self {
        let min_len = (k + 1) * min_seg;
        if len < min_len {
            return Self {
                entity_idx,
                k,
                len,
                max_nearby,
                min_seg,
                stack: vec![],
                nearby_cache: vec![],
                done: true,
            };
        }

        // Start with first valid position
        let iter = Self {
            entity_idx,
            k,
            len,
            max_nearby,
            min_seg,
            stack: vec![(min_seg, 0)],
            nearby_cache: vec![vec![]],
            done: false,
        };
        iter
    }

    fn is_done(&self) -> bool {
        self.done
    }

    fn extend_stack<S, D>(&mut self, solution: &S, distance_meter: &D)
    where
        D: ListPositionDistanceMeter<S>,
    {
        while self.stack.len() < self.k && !self.done {
            let (last_pos, _) = *self.stack.last().unwrap();

            let nearby = nearby_positions(
                solution,
                distance_meter,
                self.max_nearby,
                self.entity_idx,
                last_pos,
                self.len,
            );

            // Filter to valid positions (must leave room for remaining cuts)
            let remaining_cuts = self.k - self.stack.len();
            let min_pos = last_pos + self.min_seg;
            let max_pos = self.len - self.min_seg * remaining_cuts;

            let valid: Vec<usize> = nearby
                .into_iter()
                .filter(|&p| p >= min_pos && p <= max_pos)
                .collect();

            if valid.is_empty() {
                // No valid positions, backtrack
                if !self.backtrack() {
                    self.done = true;
                    return;
                }
            } else {
                self.nearby_cache.push(valid);
                let next_pos = self.nearby_cache.last().unwrap()[0];
                self.stack.push((next_pos, 0));
            }
        }
    }

    fn backtrack(&mut self) -> bool {
        while let Some((popped_pos, _idx)) = self.stack.pop() {
            self.nearby_cache.pop();

            if let Some((_, last_idx)) = self.stack.last_mut() {
                let cache_idx = self.nearby_cache.len();
                if cache_idx > 0 {
                    let cache = &self.nearby_cache[cache_idx - 1];
                    let next_idx = *last_idx + 1;
                    if next_idx < cache.len() {
                        *last_idx = next_idx;
                        let (pos, _) = self.stack.last().unwrap();
                        let new_pos = cache[next_idx];
                        if new_pos > *pos {
                            self.stack.pop();
                            self.stack.push((new_pos, next_idx));
                            return true;
                        }
                    }
                }
            } else {
                // Stack is empty - use the popped position to find next first position
                let next_first = popped_pos + 1;
                let max_first = self.len - self.min_seg * self.k;
                if next_first <= max_first {
                    self.stack.push((next_first, 0));
                    self.nearby_cache.push(vec![]);
                    return true;
                }
            }
        }
        false
    }

    fn advance<S, D>(&mut self, solution: &S, distance_meter: &D)
    where
        D: ListPositionDistanceMeter<S>,
    {
        if self.done || self.stack.is_empty() {
            self.done = true;
            return;
        }

        // Try to advance at current depth
        if let Some((_, idx)) = self.stack.last_mut() {
            let cache_idx = self.nearby_cache.len() - 1;
            let cache = &self.nearby_cache[cache_idx];
            let next_idx = *idx + 1;
            if next_idx < cache.len() {
                *idx = next_idx;
                let new_pos = cache[next_idx];
                self.stack.pop();
                self.stack.push((new_pos, next_idx));
                return;
            }
        }

        // Backtrack and extend
        if self.backtrack() {
            self.extend_stack(solution, distance_meter);
        } else {
            self.done = true;
        }
    }

    fn next_cuts<S, D>(&mut self, solution: &S, distance_meter: &D) -> Option<Vec<CutPoint>>
    where
        D: ListPositionDistanceMeter<S>,
    {
        self.extend_stack(solution, distance_meter);
        if self.done || self.stack.len() != self.k {
            return None;
        }

        let cuts: Vec<CutPoint> = self
            .stack
            .iter()
            .map(|(pos, _)| CutPoint::new(self.entity_idx, *pos))
            .collect();

        self.advance(solution, distance_meter);

        Some(cuts)
    }
}
