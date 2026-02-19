//! Nearby list change move selector for distance-pruned element relocation.
//!
//! A distance-biased variant of [`ListChangeMoveSelector`] that dramatically
//! reduces the move space by only considering destination positions that are
//! close to the source element. This is critical for VRP scalability: without
//! nearby selection, the move space is O(n²m²) which becomes impractical for
//! large instances.
//!
//! # How It Works
//!
//! For each source (entity, position), instead of generating moves to every
//! possible (dest_entity, dest_pos), only the `max_nearby` closest destination
//! positions are considered. Closeness is measured by a user-supplied
//! [`CrossEntityDistanceMeter`].
//!
//! # Complexity
//!
//! O(nm × k) per step where:
//! - n = number of entities
//! - m = average route length
//! - k = `max_nearby` (typically 10–20)
//!
//! Compare to O(n²m²) for the full [`ListChangeMoveSelector`].
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::nearby_list_change::{
//!     CrossEntityDistanceMeter, NearbyListChangeMoveSelector,
//! };
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
//! fn list_remove(s: &mut Solution, e: usize, pos: usize) -> Option<Visit> {
//!     s.vehicles.get_mut(e).map(|v| v.visits.remove(pos))
//! }
//! fn list_insert(s: &mut Solution, e: usize, pos: usize, val: Visit) {
//!     if let Some(v) = s.vehicles.get_mut(e) { v.visits.insert(pos, val); }
//! }
//!
//! // Euclidean distance between visit elements across routes
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
//! let selector = NearbyListChangeMoveSelector::<Solution, Visit, _, _>::new(
//!     FromSolutionEntitySelector::new(0),
//!     EuclideanMeter,
//!     10,   // max_nearby: consider 10 closest destinations
//!     list_len,
//!     list_remove,
//!     list_insert,
//!     "visits",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ListChangeMove, ListMoveImpl};

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// Measures distance between two list positions, potentially across different entities.
///
/// Used by [`NearbyListChangeMoveSelector`] to rank candidate destination positions
/// by proximity to the source element being relocated.
///
/// # Notes
///
/// - Implementing this for VRP: use the Euclidean (or road-network) distance between
///   the visit at `(src_entity, src_pos)` and the visit at `(dst_entity, dst_pos)`.
/// - The distance can be asymmetric (e.g., directed graphs).
/// - Returning `f64::INFINITY` for a pair excludes it from nearby candidates.
pub trait CrossEntityDistanceMeter<S>: Send + Sync + Debug {
    /// Returns the distance from the element at `(src_entity, src_pos)` to the element
    /// at `(dst_entity, dst_pos)` in the current solution.
    fn distance(
        &self,
        solution: &S,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> f64;
}

/// Default distance meter: uses absolute position difference within the same entity,
/// and returns `f64::INFINITY` for cross-entity distances (no pruning across routes).
///
/// Useful for intra-route moves only.
#[derive(Debug, Clone, Copy)]
pub struct DefaultCrossEntityDistanceMeter;

impl<S> CrossEntityDistanceMeter<S> for DefaultCrossEntityDistanceMeter {
    fn distance(
        &self,
        _solution: &S,
        src_entity: usize,
        src_pos: usize,
        dst_entity: usize,
        dst_pos: usize,
    ) -> f64 {
        if src_entity == dst_entity {
            (src_pos as f64 - dst_pos as f64).abs()
        } else {
            f64::INFINITY
        }
    }
}

/// A distance-pruned list change move selector.
///
/// For each source (entity, position), generates moves only to the `max_nearby`
/// nearest destination positions (measured by `CrossEntityDistanceMeter`).
/// This reduces move space from O(n²m²) to O(nm × k).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `D` - The distance meter type
/// * `ES` - The entity selector type
pub struct NearbyListChangeMoveSelector<S, V, D, ES> {
    entity_selector: ES,
    distance_meter: D,
    max_nearby: usize,
    list_len: fn(&S, usize) -> usize,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, D: Debug, ES: Debug> Debug for NearbyListChangeMoveSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NearbyListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("distance_meter", &self.distance_meter)
            .field("max_nearby", &self.max_nearby)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, D, ES> NearbyListChangeMoveSelector<S, V, D, ES> {
    /// Creates a new nearby list change move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for moves
    /// * `distance_meter` - Measures distance between position pairs
    /// * `max_nearby` - Maximum destination positions to consider per source
    /// * `list_len` - Function to get list length for an entity
    /// * `list_remove` - Function to remove element at position
    /// * `list_insert` - Function to insert element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        distance_meter: D,
        max_nearby: usize,
        list_len: fn(&S, usize) -> usize,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            distance_meter,
            max_nearby,
            list_len,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, D, ES> MoveSelector<S, ListChangeMove<S, V>>
    for NearbyListChangeMoveSelector<S, V, D, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    D: CrossEntityDistanceMeter<S>,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, SD: ScoreDirector<S>>(
        &'a self,
        score_director: &'a SD,
    ) -> impl Iterator<Item = ListChangeMove<S, V>> + 'a {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let list_remove = self.list_remove;
        let list_insert = self.list_insert;
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

        for (src_idx, &src_entity) in entities.iter().enumerate() {
            let src_len = route_lens[src_idx];
            if src_len == 0 {
                continue;
            }

            for src_pos in 0..src_len {
                // Collect all candidate (dst_entity, dst_pos) pairs with distances
                let mut candidates: Vec<(usize, usize, f64)> = Vec::new();

                // Intra-entity candidates
                for dst_pos in 0..src_len {
                    if dst_pos == src_pos || dst_pos == src_pos + 1 {
                        continue; // Skip no-ops
                    }
                    let dist = self
                        .distance_meter
                        .distance(solution, src_entity, src_pos, src_entity, dst_pos);
                    if dist.is_finite() {
                        candidates.push((src_entity, dst_pos, dist));
                    }
                }

                // Inter-entity candidates: insert at any position in other entities
                for (dst_idx, &dst_entity) in entities.iter().enumerate() {
                    if dst_idx == src_idx {
                        continue;
                    }
                    let dst_len = route_lens[dst_idx];
                    // Can insert at positions 0..=dst_len
                    for dst_pos in 0..=dst_len {
                        // For the last position, use the distance to the last element
                        let ref_pos = dst_pos.min(dst_len.saturating_sub(1));
                        let dist = self
                            .distance_meter
                            .distance(solution, src_entity, src_pos, dst_entity, ref_pos);
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
                    moves.push(ListChangeMove::new(
                        src_entity,
                        src_pos,
                        dst_entity,
                        dst_pos,
                        list_len,
                        list_remove,
                        list_insert,
                        variable_name,
                        descriptor_index,
                    ));
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

        // Each element generates at most max_nearby moves
        total_elements * self.max_nearby
    }
}

/// Wraps a `NearbyListChangeMoveSelector` to yield `ListMoveImpl::ListChange`.
pub struct ListMoveNearbyListChangeSelector<S, V, D, ES> {
    inner: NearbyListChangeMoveSelector<S, V, D, ES>,
}

impl<S, V: Debug, D: Debug, ES: Debug> Debug for ListMoveNearbyListChangeSelector<S, V, D, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveNearbyListChangeSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, D, ES> ListMoveNearbyListChangeSelector<S, V, D, ES> {
    /// Wraps an existing [`NearbyListChangeMoveSelector`].
    pub fn new(inner: NearbyListChangeMoveSelector<S, V, D, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, D, ES> MoveSelector<S, ListMoveImpl<S, V>>
    for ListMoveNearbyListChangeSelector<S, V, D, ES>
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
            .map(ListMoveImpl::ListChange)
    }

    fn size<SD: ScoreDirector<S>>(&self, score_director: &SD) -> usize {
        self.inner.size(score_director)
    }
}
