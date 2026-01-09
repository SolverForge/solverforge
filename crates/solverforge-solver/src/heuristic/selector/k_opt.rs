//! K-opt move selector for tour optimization.
//!
//! Generates k-opt moves by enumerating all valid cut point combinations
//! within selected entities and applying reconnection patterns.
//!
//! # Complexity
//!
//! For a route of length n and k-opt:
//! - Full enumeration: O(n^k) cut combinations × reconnection patterns
//! - Use `NearbyKOptMoveSelector` to reduce to O(n × m^(k-1)) with nearby selection
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::k_opt::{KOptMoveSelector, KOptConfig};
//! use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Tour { cities: Vec<i32>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Tour {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn list_len(s: &Tour, _: usize) -> usize { s.cities.len() }
//! fn sublist_remove(s: &mut Tour, _: usize, start: usize, end: usize) -> Vec<i32> {
//!     s.cities.drain(start..end).collect()
//! }
//! fn sublist_insert(s: &mut Tour, _: usize, pos: usize, items: Vec<i32>) {
//!     for (i, item) in items.into_iter().enumerate() {
//!         s.cities.insert(pos + i, item);
//!     }
//! }
//!
//! let config = KOptConfig::new(3); // 3-opt
//! let entity_selector = Box::new(FromSolutionEntitySelector::new(0));
//!
//! let selector = KOptMoveSelector::<Tour, i32>::new(
//!     entity_selector,
//!     config,
//!     list_len,
//!     sublist_remove,
//!     sublist_insert,
//!     "cities",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::k_opt_reconnection::{
    enumerate_reconnections, KOptReconnection, THREE_OPT_RECONNECTIONS,
};
use crate::heuristic::r#move::{CutPoint, KOptMove};

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// Configuration for k-opt move generation.
#[derive(Debug, Clone)]
pub struct KOptConfig {
    /// The k value (2-5).
    pub k: usize,
    /// Minimum segment length between cuts (default: 1).
    pub min_segment_len: usize,
    /// Whether to use only a subset of reconnection patterns.
    pub limited_patterns: bool,
}

impl KOptConfig {
    /// Creates a new k-opt configuration.
    ///
    /// # Panics
    ///
    /// Panics if k < 2 or k > 5.
    pub fn new(k: usize) -> Self {
        assert!((2..=5).contains(&k), "k must be between 2 and 5");
        Self {
            k,
            min_segment_len: 1,
            limited_patterns: false,
        }
    }

    /// Sets minimum segment length between cuts.
    pub fn with_min_segment_len(mut self, len: usize) -> Self {
        self.min_segment_len = len;
        self
    }

    /// Enables limited pattern mode (faster but less thorough).
    pub fn with_limited_patterns(mut self, limited: bool) -> Self {
        self.limited_patterns = limited;
        self
    }
}

/// A move selector that generates k-opt moves.
///
/// Enumerates all valid cut point combinations for each selected entity
/// and generates moves for each reconnection pattern.
pub struct KOptMoveSelector<S, V> {
    /// Selects entities (routes) to apply k-opt to.
    entity_selector: Box<dyn EntitySelector<S>>,
    /// K-opt configuration.
    config: KOptConfig,
    /// Reconnection patterns to use.
    patterns: Vec<&'static KOptReconnection>,
    /// Get list length for an entity.
    list_len: fn(&S, usize) -> usize,
    /// Remove sublist [start, end).
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    /// Insert elements at position.
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    /// Variable name.
    variable_name: &'static str,
    /// Descriptor index.
    descriptor_index: usize,
    _phantom: PhantomData<V>,
}

impl<S, V: Debug> Debug for KOptMoveSelector<S, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KOptMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("config", &self.config)
            .field("pattern_count", &self.patterns.len())
            .field("variable_name", &self.variable_name)
            .finish()
    }
}

impl<S: PlanningSolution, V> KOptMoveSelector<S, V> {
    /// Creates a new k-opt move selector.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: Box<dyn EntitySelector<S>>,
        config: KOptConfig,
        list_len: fn(&S, usize) -> usize,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        // Get static patterns for k=3, generate for others
        let patterns: Vec<&'static KOptReconnection> = if config.k == 3 {
            THREE_OPT_RECONNECTIONS.iter().collect()
        } else {
            // For other k values, we need to leak the patterns to get 'static lifetime
            // This is a one-time allocation per selector creation
            let generated = enumerate_reconnections(config.k);
            let leaked: &'static [KOptReconnection] = Box::leak(generated.into_boxed_slice());
            leaked.iter().collect()
        };

        Self {
            entity_selector,
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
}

impl<S, V> MoveSelector<S, KOptMove<S, V>> for KOptMoveSelector<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
    ) -> Box<dyn Iterator<Item = KOptMove<S, V>> + 'a> {
        let k = self.config.k;
        let min_seg = self.config.min_segment_len;
        let patterns = &self.patterns;
        let list_len = self.list_len;
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
                let len = list_len(solution, entity_idx);

                // Generate all valid cut combinations
                let cuts_iter = CutCombinationIterator::new(k, len, min_seg, entity_idx);

                cuts_iter.flat_map(move |cuts| {
                    // For each cut combination, generate moves for each pattern
                    patterns.iter().map(move |&pattern| {
                        KOptMove::new(
                            &cuts,
                            pattern,
                            list_len,
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

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
        let k = self.config.k;
        let min_seg = self.config.min_segment_len;
        let pattern_count = self.patterns.len();

        self.entity_selector
            .iter(score_director)
            .map(|entity_ref| {
                let solution = score_director.working_solution();
                let len = (self.list_len)(solution, entity_ref.entity_index);
                count_cut_combinations(k, len, min_seg) * pattern_count
            })
            .sum()
    }
}

/// Iterator over all valid k-cut combinations for a route of given length.
struct CutCombinationIterator {
    k: usize,
    len: usize,
    min_seg: usize,
    entity_idx: usize,
    /// Current cut positions.
    positions: Vec<usize>,
    /// Whether we've exhausted all combinations.
    done: bool,
}

impl CutCombinationIterator {
    fn new(k: usize, len: usize, min_seg: usize, entity_idx: usize) -> Self {
        // Minimum length required: k cuts need k+1 segments of min_seg each
        let min_len = (k + 1) * min_seg;

        if len < min_len {
            return Self {
                k,
                len,
                min_seg,
                entity_idx,
                positions: vec![],
                done: true,
            };
        }

        // Initialize with first valid combination
        // Cuts must be at positions that leave min_seg elements between them
        let mut positions = Vec::with_capacity(k);
        for i in 0..k {
            positions.push(min_seg * (i + 1));
        }

        Self {
            k,
            len,
            min_seg,
            entity_idx,
            positions,
            done: false,
        }
    }

    fn advance(&mut self) -> bool {
        if self.done || self.positions.is_empty() {
            return false;
        }

        // Find the rightmost position that can be incremented
        let k = self.k;
        let len = self.len;
        let min_seg = self.min_seg;

        for i in (0..k).rev() {
            // Maximum position for cut i:
            // Need to leave room for (k - i - 1) more cuts after this one,
            // each separated by min_seg, plus min_seg at the end
            let max_pos = len - min_seg * (k - i);

            if self.positions[i] < max_pos {
                self.positions[i] += 1;
                // Reset all positions after i
                for j in (i + 1)..k {
                    self.positions[j] = self.positions[j - 1] + min_seg;
                }
                return true;
            }
        }

        self.done = true;
        false
    }
}

impl Iterator for CutCombinationIterator {
    type Item = Vec<CutPoint>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let cuts: Vec<CutPoint> = self
            .positions
            .iter()
            .map(|&pos| CutPoint::new(self.entity_idx, pos))
            .collect();

        self.advance();

        Some(cuts)
    }
}

/// Counts the number of valid k-cut combinations for a route of length len.
fn count_cut_combinations(k: usize, len: usize, min_seg: usize) -> usize {
    // This is equivalent to C(n - (k+1)*min_seg + k, k)
    // where we're choosing k positions from the "free" slots
    let min_len = (k + 1) * min_seg;
    if len < min_len {
        return 0;
    }

    let free_slots = len - min_len + k;
    binomial(free_slots, k)
}

/// Compute binomial coefficient C(n, k).
fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }

    let k = k.min(n - k); // Use symmetry
    let mut result = 1;
    for i in 0..k {
        result = result * (n - i) / (i + 1);
    }
    result
}

/// A distance meter for list element positions.
///
/// Measures distance between elements at two positions in a list.
/// Used by `NearbyKOptMoveSelector` to limit k-opt search space.
pub trait ListPositionDistanceMeter<S>: Send + Sync + Debug {
    /// Measures distance between elements at two positions in the same entity.
    fn distance(&self, solution: &S, entity_idx: usize, pos_a: usize, pos_b: usize) -> f64;
}

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
pub struct NearbyKOptMoveSelector<S, V, D: ListPositionDistanceMeter<S>> {
    /// Selects entities (routes) to apply k-opt to.
    entity_selector: Box<dyn EntitySelector<S>>,
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
    _phantom: PhantomData<V>,
}

impl<S, V: Debug, D: ListPositionDistanceMeter<S>> Debug for NearbyKOptMoveSelector<S, V, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyKOptMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("max_nearby", &self.max_nearby)
            .field("config", &self.config)
            .field("pattern_count", &self.patterns.len())
            .finish()
    }
}

impl<S: PlanningSolution, V, D: ListPositionDistanceMeter<S>> NearbyKOptMoveSelector<S, V, D> {
    /// Creates a new nearby k-opt move selector.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: Box<dyn EntitySelector<S>>,
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

impl<S, V, D> MoveSelector<S, KOptMove<S, V>> for NearbyKOptMoveSelector<S, V, D>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    D: ListPositionDistanceMeter<S> + 'static,
{
    fn iter_moves<'a>(
        &'a self,
        score_director: &'a dyn ScoreDirector<S>,
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

    fn size(&self, score_director: &dyn ScoreDirector<S>) -> usize {
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
struct NearbyCutIterator<'a, S, V, D: ListPositionDistanceMeter<S>> {
    selector: &'a NearbyKOptMoveSelector<S, V, D>,
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

impl<'a, S: PlanningSolution, V, D: ListPositionDistanceMeter<S>> NearbyCutIterator<'a, S, V, D> {
    fn new(
        selector: &'a NearbyKOptMoveSelector<S, V, D>,
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

impl<'a, S: PlanningSolution, V, D: ListPositionDistanceMeter<S>> Iterator
    for NearbyCutIterator<'a, S, V, D>
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::r#move::Move;
    use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    use solverforge_core::domain::{EntityDescriptor, SolutionDescriptor, TypedEntityExtractor};
    use solverforge_core::score::SimpleScore;
    use solverforge_scoring::SimpleScoreDirector;
    use std::any::TypeId;

    #[derive(Clone, Debug)]
    struct Tour {
        cities: Vec<i32>,
    }

    #[derive(Clone, Debug)]
    struct TspSolution {
        tours: Vec<Tour>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TspSolution {
        type Score = SimpleScore;
        fn score(&self) -> Option<Self::Score> {
            self.score
        }
        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    fn get_tours(s: &TspSolution) -> &Vec<Tour> {
        &s.tours
    }
    fn get_tours_mut(s: &mut TspSolution) -> &mut Vec<Tour> {
        &mut s.tours
    }
    fn list_len(s: &TspSolution, entity_idx: usize) -> usize {
        s.tours.get(entity_idx).map_or(0, |t| t.cities.len())
    }
    fn sublist_remove(
        s: &mut TspSolution,
        entity_idx: usize,
        start: usize,
        end: usize,
    ) -> Vec<i32> {
        s.tours
            .get_mut(entity_idx)
            .map(|t| t.cities.drain(start..end).collect())
            .unwrap_or_default()
    }
    fn sublist_insert(s: &mut TspSolution, entity_idx: usize, pos: usize, items: Vec<i32>) {
        if let Some(t) = s.tours.get_mut(entity_idx) {
            for (i, item) in items.into_iter().enumerate() {
                t.cities.insert(pos + i, item);
            }
        }
    }

    fn create_director(
        tours: Vec<Tour>,
    ) -> SimpleScoreDirector<TspSolution, impl Fn(&TspSolution) -> SimpleScore> {
        let solution = TspSolution { tours, score: None };
        let extractor = Box::new(TypedEntityExtractor::new(
            "Tour",
            "tours",
            get_tours,
            get_tours_mut,
        ));
        let entity_desc =
            EntityDescriptor::new("Tour", TypeId::of::<Tour>(), "tours").with_extractor(extractor);
        let descriptor = SolutionDescriptor::new("TspSolution", TypeId::of::<TspSolution>())
            .with_entity(entity_desc);
        SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
    }

    #[test]
    fn cut_combination_iterator_basic() {
        // For k=3, len=8, min_seg=1:
        // We need 4 segments of length >= 1
        // Cuts can be at positions 1-7 (not 0 or 8)
        // First combination: [1, 2, 3]
        let mut iter = CutCombinationIterator::new(3, 8, 1, 0);

        let first = iter.next().unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(first[0].position(), 1);
        assert_eq!(first[1].position(), 2);
        assert_eq!(first[2].position(), 3);

        // Count total combinations
        let count = 1 + iter.count(); // +1 for first we already took
                                      // C(8 - 4 + 3, 3) = C(7, 3) = 35
        assert_eq!(count, 35);
    }

    #[test]
    fn cut_combination_too_short() {
        // Route too short for 3 cuts with min_seg=2
        // Need 4 segments * 2 = 8 elements minimum
        let mut iter = CutCombinationIterator::new(3, 6, 2, 0);
        assert!(iter.next().is_none());
    }

    #[test]
    fn binomial_coefficient() {
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(7, 3), 35);
        assert_eq!(binomial(10, 5), 252);
    }

    #[test]
    fn selector_generates_moves() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let director = create_director(tours);

        let config = KOptConfig::new(3);
        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));

        let selector = KOptMoveSelector::<TspSolution, i32>::new(
            entity_selector,
            config,
            list_len,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        let moves: Vec<_> = selector.iter_moves(&director).collect();

        // 35 cut combinations × 7 patterns = 245 moves
        assert_eq!(moves.len(), 245);

        // All moves should be doable
        for m in &moves {
            assert!(m.is_doable(&director), "Move not doable: {:?}", m);
        }
    }

    #[test]
    fn selector_size_matches_iteration() {
        let tours = vec![Tour {
            cities: vec![1, 2, 3, 4, 5, 6, 7, 8],
        }];
        let director = create_director(tours);

        let config = KOptConfig::new(3);
        let entity_selector = Box::new(FromSolutionEntitySelector::new(0));

        let selector = KOptMoveSelector::<TspSolution, i32>::new(
            entity_selector,
            config,
            list_len,
            sublist_remove,
            sublist_insert,
            "cities",
            0,
        );

        let size = selector.size(&director);
        let actual_count = selector.iter_moves(&director).count();

        assert_eq!(size, actual_count);
    }
}
