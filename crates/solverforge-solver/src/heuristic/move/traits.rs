//! Move trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;

/// A move that modifies one or more planning variables.
///
/// Moves are fully typed for maximum performance - no boxing, no virtual dispatch.
/// Undo is handled by `ScoreDirector`'s undo stack and score snapshot, not by move return values.
///
/// # Type Parameters
/// * `S` - The planning solution type
///
/// # Implementation Notes
/// - Moves should be lightweight
/// - Use `save_score_snapshot()` + `undo_changes()` for move evaluation
/// - Moves are NEVER cloned - ownership transfers via arena indices
pub trait Move<S>: Send + Sync + Debug
where
    S: PlanningSolution,
    S::Score: Score,
{
    /// Returns true if this move can be executed in the current state.
    ///
    /// A move is not doable if:
    /// - The source value equals the destination value (no change)
    /// - Required entities are pinned
    /// - The move would violate hard constraints that can be detected early
    fn is_doable<C>(&self, score_director: &ScoreDirector<S, C>) -> bool
    where
        C: solverforge_scoring::ConstraintSet<S, S::Score>;

    /// Executes this move, modifying the working solution.
    ///
    /// This method modifies the planning variables through the score director.
    fn do_move<C>(&self, score_director: &mut ScoreDirector<S, C>)
    where
        C: solverforge_scoring::ConstraintSet<S, S::Score>;

    /// Returns the descriptor index of the entity type this move affects.
    fn descriptor_index(&self) -> usize;

    /// Returns the entity indices involved in this move.
    fn entity_indices(&self) -> &[usize];

    /// Returns the variable name this move affects.
    fn variable_name(&self) -> &str;

    /// Returns move strength for WeakestFit/StrongestFit selection.
    ///
    /// Higher values indicate "stronger" moves. WeakestFit picks minimum strength,
    /// StrongestFit picks maximum strength.
    fn strength(&self) -> i64 {
        self.entity_indices().first().copied().unwrap_or(0) as i64
    }
}
