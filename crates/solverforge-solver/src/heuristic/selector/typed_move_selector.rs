//! Move selector trait for typed move generation.
//!
//! This module contains the core `MoveSelector` trait that yields moves of type `M` directly,
//! eliminating heap allocation per move through monomorphization.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::heuristic::r#move::Move;

/// A typed move selector that yields moves of type `M` directly.
///
/// Unlike erased selectors, this returns concrete moves inline,
/// eliminating heap allocation per move.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `M` - The move type
pub trait MoveSelector<S, M>: Send + Debug
where
    S: PlanningSolution,
    S::Score: Score,
    M: Move<S>,
{
    /// Returns an iterator over typed moves.
    fn iter_moves<'a, C>(
        &'a self,
        score_director: &'a ScoreDirector<S, C>,
    ) -> Box<dyn Iterator<Item = M> + 'a>
    where
        C: ConstraintSet<S, S::Score>;

    /// Returns the approximate number of moves.
    fn size<C>(&self, score_director: &ScoreDirector<S, C>) -> usize
    where
        C: ConstraintSet<S, S::Score>;

    /// Returns true if this selector may return the same move multiple times.
    fn is_never_ending(&self) -> bool {
        false
    }
}
