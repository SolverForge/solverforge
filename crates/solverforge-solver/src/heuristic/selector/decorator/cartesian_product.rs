//! Cartesian product move selector.
//!
//! Combines moves from two selectors by storing them in separate arenas
//! and yielding CompositeMove references for each pair.
//!
//! # Zero-Erasure Design
//!
//! Moves are stored in typed arenas. The cartesian product iterator
//! yields indices into both arenas. The caller creates CompositeMove
//! references on-the-fly for each evaluation - no cloning.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::{Move, MoveArena};
use crate::heuristic::selector::MoveSelector;

/// Holds two arenas of moves and provides iteration over all pairs.
///
/// This is NOT a MoveSelector - it's a specialized structure for
/// cartesian product iteration that preserves zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M1` - First move type
/// * `M2` - Second move type
pub struct CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    arena_1: MoveArena<M1>,
    arena_2: MoveArena<M2>,
    _phantom: PhantomData<S>,
}

impl<S, M1, M2> CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    S::Score: Score,
    M1: Move<S>,
    M2: Move<S>,
{
    /// Creates a new empty cartesian product arena.
    pub fn new() -> Self {
        Self {
            arena_1: MoveArena::new(),
            arena_2: MoveArena::new(),
            _phantom: PhantomData,
        }
    }

    /// Resets both arenas for the next step.
    pub fn reset(&mut self) {
        self.arena_1.reset();
        self.arena_2.reset();
    }

    /// Populates arena 1 from a move selector.
    pub fn populate_first<C, MS>(&mut self, selector: &MS, score_director: &ScoreDirector<S, C>)
    where
        C: ConstraintSet<S, S::Score>,
        MS: MoveSelector<S, M1>,
    {
        self.arena_1.extend(selector.iter_moves(score_director));
    }

    /// Populates arena 2 from a move selector.
    pub fn populate_second<C, MS>(&mut self, selector: &MS, score_director: &ScoreDirector<S, C>)
    where
        C: ConstraintSet<S, S::Score>,
        MS: MoveSelector<S, M2>,
    {
        self.arena_2.extend(selector.iter_moves(score_director));
    }

    /// Returns the number of pairs (size of cartesian product).
    pub fn len(&self) -> usize {
        self.arena_1.len() * self.arena_2.len()
    }

    /// Returns true if either arena is empty.
    pub fn is_empty(&self) -> bool {
        self.arena_1.is_empty() || self.arena_2.is_empty()
    }

    /// Returns the first move at the given index.
    pub fn get_first(&self, index: usize) -> Option<&M1> {
        self.arena_1.get(index)
    }

    /// Returns the second move at the given index.
    pub fn get_second(&self, index: usize) -> Option<&M2> {
        self.arena_2.get(index)
    }

    /// Returns an iterator over all (i, j) index pairs.
    pub fn iter_indices(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let len_1 = self.arena_1.len();
        let len_2 = self.arena_2.len();
        (0..len_1).flat_map(move |i| (0..len_2).map(move |j| (i, j)))
    }

    /// Returns an iterator over all (i, j) pairs with references to both moves.
    pub fn iter_pairs(&self) -> impl Iterator<Item = (usize, usize, &M1, &M2)> + '_ {
        self.iter_indices().filter_map(|(i, j)| {
            let m1 = self.arena_1.get(i)?;
            let m2 = self.arena_2.get(j)?;
            Some((i, j, m1, m2))
        })
    }
}

impl<S, M1, M2> Default for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    S::Score: Score,
    M1: Move<S>,
    M2: Move<S>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, M1, M2> Debug for CartesianProductArena<S, M1, M2>
where
    S: PlanningSolution,
    M1: Move<S>,
    M2: Move<S>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CartesianProductArena")
            .field("arena_1_len", &self.arena_1.len())
            .field("arena_2_len", &self.arena_2.len())
            .finish()
    }
}
