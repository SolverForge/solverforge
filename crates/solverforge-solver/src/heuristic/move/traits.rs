// Move trait definition.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::MoveTabuSignature;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveAffectedEntity<'a> {
    pub descriptor_index: usize,
    pub entity_index: usize,
    pub variable_name: &'a str,
}

/// A move that modifies one or more planning variables.
///
/// Moves are fully monomorphized for maximum performance - no boxing, no virtual dispatch.
/// Undo is handled by `RecordingDirector`, not by move return values.
///
/// # Type Parameters
/// * `S` - The planning solution type
///
/// # Implementation Notes
/// - Moves should be lightweight
/// - Use `RecordingDirector` to wrap the score director for automatic undo
/// - Moves are NEVER cloned - ownership transfers via arena indices
/// - Methods are generic over D to allow use with both concrete directors and RecordingDirector
pub trait Move<S: PlanningSolution>: Send + Sync + Debug {
    /* Returns true if this move can be executed in the current state.

    A move is not doable if:
    - The source value equals the destination value (no change)
    - Required entities are pinned
    - The move would violate hard constraints that can be detected early
    */
    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool;

    /* Executes this move, modifying the working solution.

    This method modifies the planning variables through the score director.
    Use `RecordingDirector` to enable automatic undo via `undo_changes()`.
    */
    fn do_move<D: Director<S>>(&self, score_director: &mut D);

    fn descriptor_index(&self) -> usize;

    fn entity_indices(&self) -> &[usize];

    fn variable_name(&self) -> &str;

    fn requires_hard_improvement(&self) -> bool {
        false
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature;

    fn for_each_affected_entity(&self, visitor: &mut dyn FnMut(MoveAffectedEntity<'_>)) {
        let descriptor_index = self.descriptor_index();
        let variable_name = self.variable_name();
        for &entity_index in self.entity_indices() {
            visitor(MoveAffectedEntity {
                descriptor_index,
                entity_index,
                variable_name,
            });
        }
    }
}
