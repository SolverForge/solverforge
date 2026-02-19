//! Nearby list swap move selector for distance-pruned element exchange.
//!
//! A distance-biased variant of [`ListSwapMoveSelector`] that only considers
//! swap partners within a configurable distance of the source element.
//! Reduces the move space from O(n²m²) to O(nm × k).
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::nearby_list_swap::NearbyListSwapMoveSelector;
//! use solverforge_solver::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
//! use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
//! use solverforge_solver::heuristic::selector::MoveSelector;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Visit { x: f64, y: f64 }
//!
//! #[derive(Clone, Debug)]
//! struct Vehicle { visits: Vec<Visit> }
//!
//! #[derive(Clone, Debug)]
//! struct Solution { vehicles: Vec<Vehicle>, score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Solution {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! fn list_len(s: &Solution, e: usize) -> usize {
//!     s.vehicles.get(e).map_or(0, |v| v.visits.len())
//! }
//! fn list_get(s: &Solution, e: usize, pos: usize) -> Option<Visit> {
//!     s.vehicles.get(e).and_then(|v| v.visits.get(pos).cloned())
//! }
//! fn list_set(s: &mut Solution, e: usize, pos: usize, val: Visit) {
//!     if let Some(v) = s.vehicles.get_mut(e) {
//!         if let Some(elem) = v.visits.get_mut(pos) { *elem = val; }
//!     }
//! }
//!
//! #[derive(Debug)]
//! struct EuclideanMeter;
//!
//! impl CrossEntityDistanceMeter<Solution> for EuclideanMeter {
//!     fn distance(
//!         &self,
//!         solution: &Solution,
//!         src_entity: usize, src_pos: usize,
//!         dst_entity: usize, dst_pos: usize,
//!     ) -> f64 {
//!         let src = &solution.vehicles[src_entity].visits[src_pos];
//!         let dst = &solution.vehicles[dst_entity].visits[dst_pos];
//!         let dx = src.x - dst.x;
//!         let dy = src.y - dst.y;
//!         (dx * dx + dy * dy).sqrt()
//!     }
//! }
//!
//! let selector = NearbyListSwapMoveSelector::<Solution, Visit, _, _>::new(
//!     FromSolutionEntitySelector::new(0),
//!     EuclideanMeter,
//!     10,
//!     list_len,
//!     list_get,
//!     list_set,
//!     "visits",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ListMoveImpl, ListSwapMove};

use super::entity::EntitySelector;
use super::nearby_list_change::CrossEntityDistanceMeter;
use super::typed_move_selector::MoveSelector;

/// A distance-pruned list swap move selector.
///
/// For each source (entity, position), generates swap moves only to the
/// `max_nearby` nearest positions (measured by `CrossEntityDistanceMeter`).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `D` - The cross-entity distance meter type
/// * `ES` - The entity selector type
pub struct NearbyListSwapMoveSelector<S, V, D, ES> {
    entity_selector: ES,
    distance_meter: D,
    max_nearby: usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, D: Debug, ES: Debug> Debug for NearbyListSwapMoveSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("distance_meter", &self.distance_meter)
            .field("max_nearby", &self.max_nearby)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, D, ES> NearbyListSwapMoveSelector<S, V, D, ES> {
    /// Creates a new nearby list swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for swaps
    /// * `distance_meter` - Measures distance between position pairs
    /// * `max_nearby` - Maximum partner positions to consider per source
    /// * `list_len` - Function to get list length
    /// * `list_get` - Function to get element at position
    /// * `list_set` - Function to set element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            distance_meter,
            max_nearby,
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, D, ES> MoveSelector<S, ListSwapMove<S, V>> for NearbyListSwapMoveSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, SD: ScoreDirector<S>>(
        &'a self,
        score_director: &'a SD,
    ) -> impl Iterator<Item = ListSwapMove<S, V>> + 'a {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let list_get = self.list_get;
        let list_set = self.list_set;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let max_nearby = self.max_nearby;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();

        let mut moves = Vec::new();
        // Track seen pairs to avoid duplicates (a,pa) <-> (b,pb) == (b,pb) <-> (a,pa)
        let mut seen: std::collections::HashSet<(usize, usize, usize, usize)> =
            std::collections::HashSet::new();

        for (src_idx, &src_entity) in entities.iter().enumerate() {
            let src_len = route_lens[src_idx];
            if src_len == 0 {
                continue;
            }

            for src_pos in 0..src_len {
                // Collect all candidate swap partners with distances
                let mut candidates: Vec<(usize, usize, f64)> = Vec::new();

                // Intra-entity candidates: positions after src_pos (triangular)
                for dst_pos in src_pos + 1..src_len {
                    let dist = self
                        .distance_meter
                        .distance(solution, src_entity, src_pos, src_entity, dst_pos);
                    if dist.is_finite() {
                        candidates.push((src_entity, dst_pos, dist));
                    }
                }

                // Inter-entity candidates (only entities with index > src_idx to avoid dupes)
                for (dst_idx, &dst_entity) in entities.iter().enumerate() {
                    if dst_idx <= src_idx {
                        continue;
                    }
                    let dst_len = route_lens[dst_idx];
                    if dst_len == 0 {
                        continue;
                    }

                    for dst_pos in 0..dst_len {
                        let dist = self
                            .distance_meter
                            .distance(solution, src_entity, src_pos, dst_entity, dst_pos);
                        if dist.is_finite() {
                            candidates.push((dst_entity, dst_pos, dist));
                        }
                    }
                }

                // Sort by distance, keep top max_nearby
                candidates
                    .sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
                candidates.truncate(max_nearby);

                for (dst_entity, dst_pos, _) in candidates {
                    // Canonical form: smaller entity/pos first to avoid generating reverse
                    let key = if (src_entity, src_pos) < (dst_entity, dst_pos) {
                        (src_entity, src_pos, dst_entity, dst_pos)
                    } else {
                        (dst_entity, dst_pos, src_entity, src_pos)
                    };

                    if seen.insert(key) {
                        moves.push(ListSwapMove::new(
                            src_entity,
                            src_pos,
                            dst_entity,
                            dst_pos,
                            list_len,
                            list_get,
                            list_set,
                            variable_name,
                            descriptor_index,
                        ));
                    }
                }
            }
        }

        moves.into_iter()
    }

    fn size<SD: ScoreDirector<S>>(&self, score_director: &SD) -> usize {
        let solution = score_director.working_solution();
        let list_len = self.list_len;

        let total_elements: usize = self
            .entity_selector
            .iter(score_director)
            .map(|r| list_len(solution, r.entity_index))
            .sum();

        // Each element generates at most max_nearby swap candidates
        total_elements * self.max_nearby / 2 // /2 for deduplication
    }
}

/// Wraps a `NearbyListSwapMoveSelector` to yield `ListMoveImpl::ListSwap`.
pub struct ListMoveNearbyListSwapSelector<S, V, D, ES> {
    inner: NearbyListSwapMoveSelector<S, V, D, ES>,
}

impl<S, V: Debug, D: Debug, ES: Debug> Debug for ListMoveNearbyListSwapSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveNearbyListSwapSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, D, ES> ListMoveNearbyListSwapSelector<S, V, D, ES> {
    /// Wraps an existing [`NearbyListSwapMoveSelector`].
    pub fn new(inner: NearbyListSwapMoveSelector<S, V, D, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, D, ES> MoveSelector<S, ListMoveImpl<S, V>>
    for ListMoveNearbyListSwapSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, SD: ScoreDirector<S>>(
        &'a self,
        score_director: &'a SD,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .iter_moves(score_director)
            .map(ListMoveImpl::ListSwap)
    }

    fn size<SD: ScoreDirector<S>>(&self, score_director: &SD) -> usize {
        self.inner.size(score_director)
    }
}
