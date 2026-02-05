// Score director trait definition.

use std::any::Any;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

// The score director manages solution state and score calculation.
//
// It is responsible for:
// - Maintaining the working solution
// - Calculating scores (incrementally when possible)
// - Notifying about variable changes for incremental updates
// - Managing shadow variable updates
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
    //
    // Full signature with descriptor and variable metadata.
    fn before_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    );

    // Called after a planning variable is changed.
    //
    // Full signature with descriptor and variable metadata.
    fn after_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    );

    // Simplified notification for entity change.
    //
    // Used by basic phases. Default delegates to full signature with empty metadata.
    fn before_entity_changed(&mut self, entity_index: usize) {
        self.before_variable_changed(0, entity_index, "");
    }

    // Simplified notification for entity change.
    //
    // Used by basic phases. Default delegates to full signature with empty metadata.
    fn after_entity_changed(&mut self, entity_index: usize) {
        self.after_variable_changed(0, entity_index, "");
    }

    // Triggers shadow variable listeners to update derived values.
    fn trigger_variable_listeners(&mut self);

    // Returns the number of entities for a given descriptor index.
    fn entity_count(&self, descriptor_index: usize) -> Option<usize>;

    // Returns the total number of entities across all collections.
    fn total_entity_count(&self) -> Option<usize>;

    // Gets an entity by descriptor index and entity index.
    fn get_entity(&self, descriptor_index: usize, entity_index: usize) -> Option<&dyn Any>;

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
