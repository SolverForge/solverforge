// Score director trait definition.

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

// The score director manages solution state and score calculation.
//
// It is responsible for:
// - Maintaining the working solution
// - Calculating scores (incrementally when possible)
// - Notifying about variable changes for incremental updates
// - Providing access to solution metadata via descriptors
pub trait ScoreDirector<S: PlanningSolution>: Send {
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

    // Returns true if this score director supports incremental scoring.
    fn is_incremental(&self) -> bool {
        false
    }

    // Resets the score director state.
    fn reset(&mut self) {}

    // Registers a typed undo closure.
    //
    // Called by moves after applying changes to enable automatic undo.
    // The closure will be called in reverse order during `undo_changes()`.
    //
    // Default implementation does nothing (for non-recording directors).
    fn register_undo(&mut self, _undo: Box<dyn FnOnce(&mut S) + Send>) {
        // Default: no-op - only RecordingScoreDirector stores undo closures
    }
}
