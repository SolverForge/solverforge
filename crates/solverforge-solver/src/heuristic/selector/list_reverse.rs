//! List reverse move selector for 2-opt optimization.
//!
//! Generates `ListReverseMove`s that reverse contiguous segments within a single
//! list. This is the fundamental 2-opt move for TSP and VRP: reversing a segment
//! of the tour can eliminate crossing edges and reduce total distance.
//!
//! For VRP, 2-opt is applied independently within each route (intra-route 2-opt).
//! Cross-route 2-opt would require inter-entity reversal, which is a different
//! operation modeled by `SubListSwapMove` with same-size segments.
//!
//! # Complexity
//!
//! For n entities with average route length m:
//! O(n * m²) — all (start, end) pairs per entity where end > start + 1.
//!
//! # Example
//!
//! ```
//! use solverforge_solver::heuristic::selector::list_reverse::ListReverseMoveSelector;
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
//! fn list_reverse(s: &mut Solution, entity_idx: usize, start: usize, end: usize) {
//!     if let Some(v) = s.vehicles.get_mut(entity_idx) {
//!         v.visits[start..end].reverse();
//!     }
//! }
//!
//! let selector = ListReverseMoveSelector::<Solution, i32, _>::new(
//!     FromSolutionEntitySelector::new(0),
//!     list_len,
//!     list_reverse,
//!     "visits",
//!     0,
//! );
//! ```

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{ListMoveImpl, ListReverseMove};

use super::entity::EntitySelector;
use super::typed_move_selector::MoveSelector;

/// A move selector that generates 2-opt segment reversal moves.
///
/// For each entity, enumerates all valid (start, end) pairs where
/// `end > start + 1` (at least 2 elements in the reversed segment).
///
/// # Type Parameters
/// * `S` - The solution type
/// * `V` - The list element type (phantom — only used for type safety)
/// * `ES` - The entity selector type
pub struct ListReverseMoveSelector<S, V, ES> {
    entity_selector: ES,
    list_len: fn(&S, usize) -> usize,
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<(fn() -> S, fn() -> V)>,
}

impl<S, V: Debug, ES: Debug> Debug for ListReverseMoveSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListReverseMoveSelector")
            .field("entity_selector", &self.entity_selector)
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V, ES> ListReverseMoveSelector<S, V, ES> {
    /// Creates a new list reverse move selector.
    ///
    /// # Arguments
    /// * `entity_selector` - Selects entities (routes) to apply 2-opt to
    /// * `list_len` - Function to get route length
    /// * `list_reverse` - Function to reverse `[start, end)` in-place
    /// * `variable_name` - Name of the list variable
    /// * `descriptor_index` - Entity descriptor index
    pub fn new(
        entity_selector: ES,
        list_len: fn(&S, usize) -> usize,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_selector,
            list_len,
            list_reverse,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V, ES> MoveSelector<S, ListReverseMove<S, V>> for ListReverseMoveSelector<S, V, ES>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
    ES: EntitySelector<S>,
{
    fn iter_moves<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> impl Iterator<Item = ListReverseMove<S, V>> + 'a {
        let solution = score_director.working_solution();
        let list_len = self.list_len;
        let list_reverse = self.list_reverse;
        let variable_name = self.variable_name;
        let descriptor_index = self.descriptor_index;

        let entities: Vec<usize> = self
            .entity_selector
            .iter(score_director)
            .map(|r| r.entity_index)
            .collect();

        let mut moves = Vec::new();

        for &entity in &entities {
            let len = list_len(solution, entity);
            if len < 2 {
                continue;
            }

            // Enumerate all (start, end) pairs where end > start + 1
            // This covers all 2-opt reversals within this entity's list
            for start in 0..len {
                // end is exclusive; minimum valid end = start + 2
                for end in (start + 2)..=len {
                    moves.push(ListReverseMove::new(
                        entity,
                        start,
                        end,
                        list_len,
                        list_reverse,
                        variable_name,
                        descriptor_index,
                    ));
                }
            }
        }

        moves.into_iter()
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        let solution = score_director.working_solution();
        let list_len = self.list_len;

        self.entity_selector
            .iter(score_director)
            .map(|r| {
                let m = list_len(solution, r.entity_index);
                // Number of valid (start, end) pairs: m*(m-1)/2 - m = m*(m-1)/2 - m
                // For start in 0..m, end in start+2..=m: sum = m*(m-1)/2
                if m >= 2 {
                    m * (m - 1) / 2
                } else {
                    0
                }
            })
            .sum()
    }
}

/// Wraps a `ListReverseMoveSelector` to yield `ListMoveImpl::ListReverse`.
pub struct ListMoveListReverseSelector<S, V, ES> {
    inner: ListReverseMoveSelector<S, V, ES>,
}

impl<S, V: Debug, ES: Debug> Debug for ListMoveListReverseSelector<S, V, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListMoveListReverseSelector")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<S, V, ES> ListMoveListReverseSelector<S, V, ES> {
    /// Wraps an existing [`ListReverseMoveSelector`].
    pub fn new(inner: ListReverseMoveSelector<S, V, ES>) -> Self {
        Self { inner }
    }
}

impl<S, V, ES> MoveSelector<S, ListMoveImpl<S, V>> for ListMoveListReverseSelector<S, V, ES>
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
            .map(ListMoveImpl::ListReverse)
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.inner.size(score_director)
    }
}
