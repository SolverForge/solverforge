//! Move trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

/// A move that modifies one or more planning variables.
///
/// Moves are fully typed for maximum performance - no boxing, no virtual dispatch.
/// Undo is handled by `RecordingScoreDirector`, not by move return values.
///
/// # Type Parameters
/// * `S` - The planning solution type
///
/// # Implementation Notes
/// - Moves should be lightweight
/// - Use `RecordingScoreDirector` to wrap the score director for automatic undo
/// - Moves are NEVER cloned - ownership transfers via arena indices
/// - Methods are generic over D to allow use with both concrete directors and RecordingScoreDirector
pub trait Move<S: PlanningSolution>: Send + Sync + Debug {
    /// Returns true if this move can be executed in the current state.
    ///
    /// A move is not doable if:
    /// - The source value equals the destination value (no change)
    /// - Required entities are pinned
    /// - The move would violate hard constraints that can be detected early
    fn is_doable<D: ScoreDirector<S>>(&self, score_director: &D) -> bool;

    /// Executes this move, modifying the working solution.
    ///
    /// This method modifies the planning variables through the score director.
    /// Use `RecordingScoreDirector` to enable automatic undo via `undo_changes()`.
    fn do_move<D: ScoreDirector<S>>(&self, score_director: &mut D);

    /// Returns the descriptor index of the entity type this move affects.
    fn descriptor_index(&self) -> usize;

    /// Returns the entity indices involved in this move.
    fn entity_indices(&self) -> &[usize];

    /// Returns the variable name this move affects.
    fn variable_name(&self) -> &str;
}
