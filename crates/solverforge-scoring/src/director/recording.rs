//! Recording score director for automatic undo tracking.
//!
//! The `RecordingScoreDirector` wraps an existing score director and stores
//! typed undo closures registered by moves. This enables zero-erasure undo:
//!
//! ```text
//! // Pattern:
//! let mut recording = RecordingScoreDirector::new(&mut inner_sd);
//! move.do_move(&mut recording);  // Move registers typed undo closure
//! let score = recording.calculate_score();
//! recording.undo_changes();  // Calls undo closures in reverse order
//! ```
//!
//! Moves capture old values using typed getters and register undo closures
//! via `register_undo()`. No BoxedValue, no type erasure on the undo path.

use std::any::Any;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use super::ScoreDirector;

/// A score director wrapper that stores typed undo closures.
///
/// Moves register their own typed undo closures via `register_undo()`.
/// This enables zero-erasure undo - no BoxedValue, no downcasting.
///
/// # Example
///
/// ```
/// use solverforge_scoring::director::{RecordingScoreDirector, SimpleScoreDirector, ScoreDirector};
/// use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
/// use solverforge_core::score::SimpleScore;
/// use std::any::TypeId;
///
/// #[derive(Clone)]
/// struct Solution { value: i32, score: Option<SimpleScore> }
///
/// impl PlanningSolution for Solution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let mut sd = SimpleScoreDirector::new(
///     Solution { value: 10, score: None },
///     SolutionDescriptor::new("Solution", TypeId::of::<Solution>()),
///     |s: &Solution| SimpleScore::of(s.value as i64),
/// );
///
/// // Wrap in recording director
/// let mut recording = RecordingScoreDirector::new(&mut sd);
///
/// // Make a change and register undo
/// let old_value = recording.working_solution().value;
/// recording.working_solution_mut().value = 20;
/// recording.register_undo(Box::new(move |s| s.value = old_value));
///
/// assert_eq!(recording.working_solution().value, 20);
///
/// // Undo restores the original value
/// recording.undo_changes();
/// assert_eq!(recording.working_solution().value, 10);
/// ```
pub struct RecordingScoreDirector<'a, S: PlanningSolution> {
    inner: &'a mut dyn ScoreDirector<S>,
    /// Typed undo closures registered by moves.
    undo_stack: Vec<Box<dyn FnOnce(&mut S) + Send>>,
}

impl<'a, S: PlanningSolution> RecordingScoreDirector<'a, S> {
    /// Creates a new recording score director wrapping the inner director.
    pub fn new(inner: &'a mut dyn ScoreDirector<S>) -> Self {
        Self {
            inner,
            undo_stack: Vec::with_capacity(16),
        }
    }

    /// Undoes all recorded changes in reverse order.
    ///
    /// Calls each undo closure in reverse order, restoring the solution
    /// to its state before any moves were applied.
    pub fn undo_changes(&mut self) {
        // Process in reverse order
        while let Some(undo) = self.undo_stack.pop() {
            undo(self.inner.working_solution_mut());
        }
    }

    /// Resets the recording state for reuse.
    ///
    /// Call this at the start of each step to reuse the Vec allocations.
    pub fn reset(&mut self) {
        self.undo_stack.clear();
    }

    /// Returns the number of recorded undo closures.
    pub fn change_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns true if there are no recorded changes.
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }
}

impl<S: PlanningSolution> ScoreDirector<S> for RecordingScoreDirector<'_, S> {
    fn working_solution(&self) -> &S {
        self.inner.working_solution()
    }

    fn working_solution_mut(&mut self) -> &mut S {
        self.inner.working_solution_mut()
    }

    fn calculate_score(&mut self) -> S::Score {
        self.inner.calculate_score()
    }

    fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.inner.solution_descriptor()
    }

    fn clone_working_solution(&self) -> S {
        self.inner.clone_working_solution()
    }

    fn before_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    ) {
        // Forward to inner for incremental scoring
        self.inner
            .before_variable_changed(descriptor_index, entity_index, variable_name);
    }

    fn after_variable_changed(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        variable_name: &str,
    ) {
        // Forward to inner for incremental scoring
        self.inner
            .after_variable_changed(descriptor_index, entity_index, variable_name);
    }

    fn trigger_variable_listeners(&mut self) {
        self.inner.trigger_variable_listeners();
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.inner.entity_count(descriptor_index)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.inner.total_entity_count()
    }

    fn get_entity(&self, descriptor_index: usize, entity_index: usize) -> Option<&dyn Any> {
        self.inner.get_entity(descriptor_index, entity_index)
    }

    fn is_incremental(&self) -> bool {
        self.inner.is_incremental()
    }

    fn reset(&mut self) {
        // Forward to inner
        self.inner.reset();
        // Also clear our undo stack
        self.undo_stack.clear();
    }

    fn register_undo(&mut self, undo: Box<dyn FnOnce(&mut S) + Send>) {
        self.undo_stack.push(undo);
    }
}
