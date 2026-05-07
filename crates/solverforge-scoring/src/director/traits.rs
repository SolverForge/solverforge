// Score director trait definition.

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::ConstraintRef;

use crate::api::constraint_set::ConstraintMetadata;

/// Snapshot of the director's committed score state.
///
/// Construction and local-search trial evaluation use this to restore the
/// wrapped director after speculative scoring.
#[derive(Debug, PartialEq, Eq)]
pub struct DirectorScoreState<Sc> {
    pub solution_score: Option<Sc>,
    pub committed_score: Option<Sc>,
    pub initialized: bool,
}

/* The score director manages solution state and score calculation.

It is responsible for:
- Maintaining the working solution
- Calculating scores (incrementally when possible)
- Notifying about variable changes for incremental updates
- Providing access to solution metadata via descriptors
*/
pub trait Director<S: PlanningSolution>: Send {
    // Returns a reference to the working solution.
    fn working_solution(&self) -> &S;

    // Returns a mutable reference to the working solution.
    fn working_solution_mut(&mut self) -> &mut S;

    // Calculates and returns the current score.
    fn calculate_score(&mut self) -> S::Score;

    // Returns the solution descriptor for this solution type.
    fn solution_descriptor(&self) -> &SolutionDescriptor;

    // Clones the working solution.
    fn clone_working_solution(&self) -> S;

    // Called before a planning variable is changed.
    fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize);

    // Called after a planning variable is changed.
    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize);

    // Returns the number of entities for a given descriptor index.
    fn entity_count(&self, descriptor_index: usize) -> Option<usize>;

    // Returns the total number of entities across all collections.
    fn total_entity_count(&self) -> Option<usize>;

    // Returns immutable scoring-constraint metadata known to this director.
    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>>;

    // Returns whether a known constraint is hard.
    fn constraint_is_hard(&self, constraint_ref: &ConstraintRef) -> Option<bool> {
        let metadata = self.constraint_metadata();
        metadata
            .iter()
            .find(|metadata| metadata.constraint_ref == constraint_ref)
            .map(|metadata| metadata.is_hard)
    }

    // Returns true if this score director supports incremental scoring.
    fn is_incremental(&self) -> bool {
        false
    }

    // Snapshots the committed score state so speculative evaluation can roll
    // back exactly.
    fn snapshot_score_state(&self) -> DirectorScoreState<S::Score> {
        let solution_score = self.working_solution().score();
        DirectorScoreState {
            solution_score,
            committed_score: solution_score,
            initialized: solution_score.is_some(),
        }
    }

    // Restores a previously snapshotted committed score state.
    fn restore_score_state(&mut self, state: DirectorScoreState<S::Score>) {
        self.working_solution_mut().set_score(state.solution_score);
    }

    // Resets the score director state.
    fn reset(&mut self) {}

    /* Registers a concrete undo closure.

    Called by moves after applying changes to enable automatic undo.
    The closure will be called in reverse order during `undo_changes()`.

    Default implementation does nothing (for non-recording directors).
    */
    fn register_undo(&mut self, _undo: Box<dyn FnOnce(&mut S) + Send>) {
        // Default: no-op - only RecordingDirector stores undo closures
    }
}
