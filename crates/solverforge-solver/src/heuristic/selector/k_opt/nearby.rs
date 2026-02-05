//! Nearby k-opt move selector with distance-based pruning.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS,
};
use crate::heuristic::r#move::{CutPoint, KOptMove};
use crate::heuristic::selector::entity::EntitySelector;
use crate::heuristic::selector::typed_move_selector::MoveSelector;

use super::config::KOptConfig;
use super::distance::ListPositionDistanceMeter;

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
    /// Selects entities (routes) to apply k-opt to.
    entity_selector: ES,
    /// Distance meter for nearby selection.
    distance_meter: D,
    /// Maximum nearby positions to consider.
    max_nearby: usize,
    /// K-opt configuration.
    config: KOptConfig,
    /// Reconnection patterns.
    patterns: Vec<&'static KOptReconnection>,
    /// Get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist.
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert sublist.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Variable name.
    variable_name: &'static str,
    /// Descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<(S, V)>,
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
    /// Creates a new nearby k-opt move selector.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        config: KOptConfig,
        list_len: fn(&S, usize) -> usize,
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
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    /// Finds the m nearest positions to a given position.
    fn nearby_positions(
        &self,
        solution: &S,
        entity_idx: usize,
        origin: usize,
        len: usize,
    ) -> Vec<usize> {
        let mut positions: Vec<(usize, f64)> = (0..len)
            .filter(|&p| p != origin)
            .map(|p| {
                let dist = self
                    .distance_meter
                    .distance(solution, entity_idx, origin, p);
                (p, dist)
            })
            .collect();

        positions.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        positions.truncate(self.max_nearby);
        positions.into_iter().map(|(p, _)| p).collect()
    }
}

impl<S, V, DM, ES> MoveSelector<S, KOptMove<S, V>> for NearbyKOptMoveSelector<S, V, DM, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    DM: ListPositionDistanceMeter<S> + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, SD: ScoreDirector<S>>(
        &'a self,
        score_director: &'a SD,
    ) -> Box<dyn Iterator<Item = KOptMove<S, V>> + 'a> {
        let k = self.config.k;
        let min_seg = self.config.min_segment_len;
        let patterns = &self.patterns;
        let list_len_fn = self.list_len;
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        let iter = self
            .entity_selector
            .iter(score_director)
            .flat_map(move |entity_ref| {
                let entity_idx = entity_ref.entity_index;
                let solution = score_director.working_solution();
                let len = list_len_fn(solution, entity_idx);

                // Generate nearby cut combinations
                let cuts_iter = NearbyCutIterator::new(self, solution, entity_idx, k, len, min_seg);

                cuts_iter.flat_map(move |cuts| {
                    patterns.iter().map(move |&pattern| {
                        // Validate cuts are sorted for intra-route
                        let mut sorted_cuts = cuts.clone();
                        sorted_cuts.sort_by_key(|c| c.position());

                        KOptMove::new(
                            &sorted_cuts,
                            pattern,
                            list_len_fn,
                            sublist_remove,
                            sublist_insert,
                            variable_name,
                            descriptor_index,
                        )
                    })
                })
            });

        Box::new(iter)
    }

    fn size<SD: ScoreDirector<S>>(&self, score_director: &SD) -> usize {
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

/// Iterator for nearby k-cut combinations.
struct NearbyCutIterator<'a, S, V, D: ListPositionDistanceMeter<S>, ES> {
    selector: &'a NearbyKOptMoveSelector<S, V, D, ES>,
    solution: &'a S,
    entity_idx: usize,
    k: usize,
    len: usize,
    min_seg: usize,
    /// Stack of (position, nearby_iterator_index)
    stack: Vec<(usize, usize)>,
    /// Nearby positions for each level
    nearby_cache: Vec<Vec<usize>>,
    done: bool,
}

impl<'a, S: PlanningSolution, V, D: ListPositionDistanceMeter<S>, ES>
    NearbyCutIterator<'a, S, V, D, ES>
{
    fn new(
        selector: &'a NearbyKOptMoveSelector<S, V, D, ES>,
        solution: &'a S,
        entity_idx: usize,
        k: usize,
        len: usize,
        min_seg: usize,
    ) -> Self {
        let min_len = (k + 1) * min_seg;
        if len < min_len {
            return Self {
                selector,
                solution,
                entity_idx,
                k,
                len,
                min_seg,
                stack: vec![],
                nearby_cache: vec![],
                done: true,
            };
        }

        // Start with first valid position
        let mut iter = Self {
            selector,
            solution,
            entity_idx,
            k,
            len,
            min_seg,
            stack: vec![(min_seg, 0)],
            nearby_cache: vec![vec![]],
            done: false,
        };

        // Build initial stack to depth k
        iter.extend_stack();
        iter
    }

    fn extend_stack(&mut self) {
        while self.stack.len() < self.k && !self.done {
            let (last_pos, _) = *self.stack.last().unwrap();

            // Get nearby positions for next cut
            let nearby =
                self.selector
                    .nearby_positions(self.solution, self.entity_idx, last_pos, self.len);

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

    fn advance(&mut self) {
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
            self.extend_stack();
        } else {
            self.done = true;
        }
    }
}

impl<'a, S: PlanningSolution, V, D: ListPositionDistanceMeter<S>, ES> Iterator
    for NearbyCutIterator<'a, S, V, D, ES>
{
    type Item = Vec<CutPoint>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.stack.len() != self.k {
            return None;
        }

        let cuts: Vec<CutPoint> = self
            .stack
            .iter()
            .map(|(pos, _)| CutPoint::new(self.entity_idx, *pos))
            .collect();

        self.advance();

        Some(cuts)
    }
}
