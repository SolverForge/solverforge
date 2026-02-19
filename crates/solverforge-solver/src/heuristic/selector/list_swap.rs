//! List swap move selector for element exchange.
//!
//! Generates `ListSwapMove`s that swap elements within or between list variables.
//! Useful for inter-route rebalancing in vehicle routing problems.
//!
//! # Complexity
//!
//! For n entities with average route length m:
//! - Intra-entity swaps: O(n * m * (m-1) / 2)
//! - Inter-entity swaps: O(n² * m²)
//! - Total: O(n² * m²) worst case (triangular optimization halves constant)
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::list_swap::ListSwapMoveSelector;
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
//! fn list_get(s: &Solution, entity_idx: usize, pos: usize) -> Option<i32> {
//!     s.vehicles.get(entity_idx).and_then(|v| v.visits.get(pos).copied())
//! }
//! fn list_set(s: &mut Solution, entity_idx: usize, pos: usize, val: i32) {
//!     if let Some(v) = s.vehicles.get_mut(entity_idx) {
//!         if let Some(elem) = v.visits.get_mut(pos) { *elem = val; }
//!     }
//! }
//!
//! let selector = ListSwapMoveSelector::<Solution, i32, _>::new(
//!     FromSolutionEntitySelector::new(0),
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
use super::typed_move_selector::MoveSelector;

/// A move selector that generates list swap moves.
///
/// Enumerates all valid (entity_a, pos_a, entity_b, pos_b) pairs for swapping
/// elements within or between list variables. Intra-entity swaps use a
/// triangular iteration to avoid duplicate pairs.
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type
/// * `ES` - The entity selector type
pub struct ListSwapMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug> Debug for ListSwapMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListSwapMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListSwapMoveSelector<S, V, ES> {
    /// Creates a new list swap move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities to consider for swaps
    /// * `list_len` - Function to get list length for an entity
    /// * `list_get` - Function to get element at position
    /// * `list_set` - Function to set element at position
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListSwapMove<S, V>> for ListSwapMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = ListSwapMove<S, V>> + 'a {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let list_get = self.list_get;
        let list_set = self.list_set;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let route_lens: Vec<usize> = entities.iter().map(|&e| list_len(solution, e)).collect();

        let mut moves = Vec::new();

        for (i, &entity_a) in entities.iter().enumerate() {
            let len_a = route_lens[i];
            if len_a == 0 {
                continue;
            }

            // Intra-entity swaps: triangular pairs (pos_a, pos_b) with pos_a < pos_b
            for pos_a in 0..len_a {
                for pos_b in pos_a + 1..len_a {
                    moves.push(ListSwapMove::new(
                        entity_a,
                        pos_a,
                        entity_a,
                        pos_b,
                        list_len,
                        list_get,
                        list_set,
                        variable_name,
                        descriptor_index,
                    ));
                }
            }

            // Inter-entity swaps: all pairs (entity_a, pos_a) x (entity_b, pos_b) where b > a
            for (j, &entity_b) in entities.iter().enumerate() {
                if j <= i {
                    continue;
                }
                let len_b = route_lens[j];
                if len_b == 0 {
                    continue;
                }

                for pos_a in 0..len_a {
                    for pos_b in 0..len_b {
                        moves.push(ListSwapMove::new(
                            entity_a,
                            pos_a,
                            entity_b,
                            pos_b,
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

        // Intra: sum of m*(m-1)/2 per entity
        // Inter: sum over pairs of m_a * m_b
        let intra: usize = route_lens
            .iter()
            .map(|&m| m * m.saturating_sub(1) / 2)
            .sum();
        let inter: usize = (0..n)
            .flat_map(|i| (i + 1..n).map(move |j| (i, j)))
            .map(|(i, j)| route_lens[i] * route_lens[j])
            .sum();
        intra + inter
    }
}

/// Wraps a `ListSwapMoveSelector` to yield `ListMoveImpl::ListSwap`.
pub struct ListMoveListSwapSelector<S, V, ES> {
    inner: ListSwapMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveListSwapSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveListSwapSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveListSwapSelector<S, V, ES> {
    /// Wraps an existing [`ListSwapMoveSelector`].
    pub fn new(inner: ListSwapMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveListSwapSelector<S, V, ES>
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
            .map(ListMoveImpl::ListSwap)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
