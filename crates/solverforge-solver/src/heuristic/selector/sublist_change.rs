//! Sublist change move selector for segment relocation (Or-opt).
//!
//! Generates `SubListChangeMove`s that relocate contiguous segments within or
//! between list variables. The Or-opt family of moves (segments of size 1, 2, 3, …)
//! is among the most effective VRP improvements after basic 2-opt.
//!
//! # Complexity
//!
//! For n entities with average route length m and max segment size k:
//! - Intra-entity: O(n * m * k) sources × O(m) destinations
//! - Inter-entity: O(n * m * k) sources × O(n * m) destinations
//! - Total: O(n² * m² * k)
//!
//! Use a forager that quits early (`FirstAccepted`, `AcceptedCount`) to keep
//! iteration practical for large instances.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::sublist_change::SubListChangeMoveSelector;
//! use solverforge_solver::heuristic::selector::entity::FromSolutionEntitySelector;
//! use solverforge_solver::heuristic::selector::MoveSelector;
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//!
//! #[derive(Clone, Debug)]
//! struct Vehicle { visits: Vec<i32> }
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
//! fn list_len(s: &Solution, entity_idx: usize) -> usize {
//!     s.vehicles.get(entity_idx).map_or(0, |v| v.visits.len())
//! }
//! fn sublist_remove(s: &mut Solution, entity_idx: usize, start: usize, end: usize) -> Vec<i32> {
//!     s.vehicles.get_mut(entity_idx)
//!         .map(|v| v.visits.drain(start..end).collect())
//!         .unwrap_or_default()
//! }
//! fn sublist_insert(s: &mut Solution, entity_idx: usize, pos: usize, items: Vec<i32>) {
//!     if let Some(v) = s.vehicles.get_mut(entity_idx) {
//!         for (i, item) in items.into_iter().enumerate() {
//!             v.visits.insert(pos + i, item);
//!         }
//!     }
//! }
//!
//! // Or-opt: relocate segments of size 1..=3
//! let selector = SubListChangeMoveSelector::<Solution, i32, _>::new(
//!     FromSolutionEntitySelector::new(0),
//!     1, 3,
//!     list_len,
//!     sublist_remove,
//!     sublist_insert,
//!     "visits",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ListMoveImpl, SubListChangeMove};

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates sublist change (Or-opt) moves.
///
/// For each source entity and each valid segment `[start, start+len)`, generates
/// moves that insert the segment at every valid destination position in every
/// entity (including the source entity for intra-route relocation).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
pub struct SubListChangeMoveSelector<S, V, ES> {
    entity_selector: ES,
    /// Minimum segment size (inclusive). Usually 1.
    min_sublist_size: usize,
    /// Maximum segment size (inclusive). Usually 3-5.
    max_sublist_size: usize,
    list_len: fn(&S, usize) -> usize,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug> Debug for SubListChangeMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListChangeMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SubListChangeMoveSelector<S, V, ES> {
    /// Creates a new sublist change move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to generate moves for
    /// * `min_sublist_size` - Minimum segment length (must be ≥ 1)
    /// * `max_sublist_size` - Maximum segment length
    /// * `list_len` - Function to get list length
    /// * `sublist_remove` - Function to drain a range `[start, end)`, returning removed elements
    /// * `sublist_insert` - Function to insert a slice at a position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    ///
    /// # Panics
    /// Panics if `min_sublist_size == 0` or `max_sublist_size < min_sublist_size`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_selector: ES,
        min_sublist_size: usize,
        max_sublist_size: usize,
        list_len: fn(&S, usize) -> usize,
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
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, SubListChangeMove<S, V>> for SubListChangeMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = SubListChangeMove<S, V>> + 'a {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let sublist_remove = self.sublist_remove;
        let sublist_insert = self.sublist_insert;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;
        let min_seg = self.min_sublist_size;
        let max_seg = self.max_sublist_size;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();

        let mut moves = Vec::new();

        for (src_idx, &src_entity) in entities.iter().enumerate() {
            let src_len = route_lens[src_idx];

            // Enumerate all valid source segments [start, end)
            for seg_start in 0..src_len {
                for seg_size in min_seg..=max_seg {
                    let seg_end = seg_start + seg_size;
                    if seg_end > src_len {
                        break; // No larger segments fit at this start
                    }

                    // Intra-entity destinations: insert at positions in the post-removal list
                    // Post-removal list has src_len - seg_size elements.
                    // Valid insertion points: 0..=(src_len - seg_size)
                    let post_removal_len = src_len - seg_size;
                    for dst_pos in 0..=post_removal_len {
                        // Skip no-ops: inserting at the same logical position
                        // After removal, seg_start..seg_end are gone.
                        // dst_pos == seg_start means insert right where we removed (no-op).
                        if dst_pos == seg_start {
                            continue;
                        }
                        moves.push(SubListChangeMove::new(
                            src_entity,
                            seg_start,
                            seg_end,
                            src_entity,
                            dst_pos,
                            list_len,
                            sublist_remove,
                            sublist_insert,
                            variable_name,
                            descriptor_index,
                        ));
                    }

                    // Inter-entity destinations
                    for (dst_idx, &dst_entity) in entities.iter().enumerate() {
                        if dst_idx == src_idx {
                            continue;
                        }
                        let dst_len = route_lens[dst_idx];
                        // Can insert at positions 0..=dst_len
                        for dst_pos in 0..=dst_len {
                            moves.push(SubListChangeMove::new(
                                src_entity,
                                seg_start,
                                seg_end,
                                dst_entity,
                                dst_pos,
                                list_len,
                                sublist_remove,
                                sublist_insert,
                                variable_name,
                                descriptor_index,
                            ));
                        }
                    }
                }
            }
        }

        moves.into_iter()
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let list_len = self.list_len;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();
        let n = entities.len();
        if n == 0 {
            return 0;
        }

        let k_range = self.max_sublist_size - self.min_sublist_size + 1;
        let total_elements: usize = route_lens.iter().sum();
        let avg_len = total_elements / n;
        // Rough estimate: n * avg_len * k_range * (avg_len + (n-1) * avg_len)
        n * avg_len * k_range * avg_len.max(1) * n
    }
}

/// Wraps a `SubListChangeMoveSelector` to yield `ListMoveImpl::SubListChange`.
pub struct ListMoveSubListChangeSelector<S, V, ES> {
    inner: SubListChangeMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveSubListChangeSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveSubListChangeSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveSubListChangeSelector<S, V, ES> {
    /// Wraps an existing [`SubListChangeMoveSelector`].
    pub fn new(inner: SubListChangeMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveSubListChangeSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = ListMoveImpl<S, V>> + 'a {
        self.inner
            .iter_moves(score_director)
            .map(ListMoveImpl::SubListChange)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
