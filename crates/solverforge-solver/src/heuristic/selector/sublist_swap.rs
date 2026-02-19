//! Sublist swap move selector for segment exchange.
//!
//! Generates `SubListSwapMove`s that swap contiguous segments within or between
//! list variables. Useful for balanced inter-route segment exchanges in VRP.
//!
//! # Complexity
//!
//! For n entities with average route length m and max segment size k:
//! - Intra-entity pairs: O(n * m² * k²) — triangular over non-overlapping segments
//! - Inter-entity pairs: O(n² * m² * k²) — all pairs across entities
//!
//! Use a forager that quits early for large instances.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::sublist_swap::SubListSwapMoveSelector;
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
//! // Swap segments of size 1..=3 between routes
//! let selector = SubListSwapMoveSelector::<Solution, i32, _>::new(
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

use crate::heuristic::r#move::{ListMoveImpl, SubListSwapMove};

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates sublist swap moves.
///
/// For each pair of segments (which may span different entities), generates
/// a swap move. Intra-entity swaps require non-overlapping segments.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
pub struct SubListSwapMoveSelector<S, V, ES> {
    entity_selector: ES,
    /// Minimum segment size (inclusive).
    min_sublist_size: usize,
    /// Maximum segment size (inclusive).
    max_sublist_size: usize,
    list_len: fn(&S, usize) -> usize,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug> Debug for SubListSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SubListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("min_sublist_size", &self.min_sublist_size)
            .field("max_sublist_size", &self.max_sublist_size)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> SubListSwapMoveSelector<S, V, ES> {
    /// Creates a new sublist swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for swaps
    /// * `min_sublist_size` - Minimum segment length (must be ≥ 1)
    /// * `max_sublist_size` - Maximum segment length
    /// * `list_len` - Function to get list length for an entity
    /// * `sublist_remove` - Function to drain range `[start, end)`, returning elements
    /// * `sublist_insert` - Function to insert elements at a position
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

impl<S, V, ES> MoveSelector<S, SubListSwapMove<S, V>> for SubListSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = SubListSwapMove<S, V>> + 'a {
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

        for (i, &entity_a) in entities.iter().enumerate() {
            let len_a = route_lens[i];

            // Intra-entity: pairs of non-overlapping segments in entity_a
            // Enumerate all valid first segments, then all non-overlapping second segments
            for first_start in 0..len_a {
                for first_size in min_seg..=max_seg {
                    let first_end = first_start + first_size;
                    if first_end > len_a {
                        break;
                    }

                    // Second segment must not overlap: second_start >= first_end
                    for second_start in first_end..len_a {
                        for second_size in min_seg..=max_seg {
                            let second_end = second_start + second_size;
                            if second_end > len_a {
                                break;
                            }
                            moves.push(SubListSwapMove::new(
                                entity_a,
                                first_start,
                                first_end,
                                entity_a,
                                second_start,
                                second_end,
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

            // Inter-entity: all segment pairs between entity_a and entity_b (b > a)
            for (j, &entity_b) in entities.iter().enumerate() {
                if j <= i {
                    continue;
                }
                let len_b = route_lens[j];
                if len_b == 0 {
                    continue;
                }

                for first_start in 0..len_a {
                    for first_size in min_seg..=max_seg {
                        let first_end = first_start + first_size;
                        if first_end > len_a {
                            break;
                        }

                        for second_start in 0..len_b {
                            for second_size in min_seg..=max_seg {
                                let second_end = second_start + second_size;
                                if second_end > len_b {
                                    break;
                                }
                                moves.push(SubListSwapMove::new(
                                    entity_a,
                                    first_start,
                                    first_end,
                                    entity_b,
                                    second_start,
                                    second_end,
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
        // Rough estimate: n * avg_len * k * (avg_len * k + (n-1) * avg_len * k) / 2
        n * avg_len * k_range * avg_len.max(1) * k_range * (n + 1) / 2
    }
}

/// Wraps a `SubListSwapMoveSelector` to yield `ListMoveImpl::SubListSwap`.
pub struct ListMoveSubListSwapSelector<S, V, ES> {
    inner: SubListSwapMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveSubListSwapSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveSubListSwapSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveSubListSwapSelector<S, V, ES> {
    /// Wraps an existing [`SubListSwapMoveSelector`].
    pub fn new(inner: SubListSwapMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveSubListSwapSelector<S, V, ES>
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
            .map(ListMoveImpl::SubListSwap)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
