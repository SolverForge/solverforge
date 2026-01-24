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

use std::marker::PhantomData;

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::Score;

use crate::api::constraint_set::ConstraintSet;
use crate::director::score_director::ScoreDirector;

/// A score director wrapper that stores typed undo closures.
///
/// Moves register their own typed undo closures via `register_undo()`.
/// This enables zero-erasure undo - no BoxedValue, no downcasting.
///
/// # Type Parameters
/// * `'a` - Lifetime of the inner score director reference
/// * `S` - The planning solution type
/// * `C` - The constraint set type
pub struct RecordingScoreDirector<'a, S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + Send,
{
    inner: &'a mut ScoreDirector<S, C>,
    /// Typed undo closures registered by moves.
    undo_stack: Vec<Box<dyn FnOnce(&mut S) + Send>>,
    /// Elements modified during this step that need shadow refresh after undo.
    /// Stores (descriptor_index, entity_index, position, element_idx) tuples.
    modified_elements: Vec<(usize, usize, usize, usize)>,
    /// Phantom data for solution type
    _phantom: PhantomData<fn() -> S>,
}

impl<'a, S, C> RecordingScoreDirector<'a, S, C>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score> + Send,
{
    /// Creates a new recording score director wrapping the inner director.
    pub fn new(inner: &'a mut ScoreDirector<S, C>) -> Self {
        Self {
            inner,
            undo_stack: Vec::with_capacity(16),
            modified_elements: Vec::with_capacity(8),
            _phantom: PhantomData,
        }
    }

    /// Returns a reference to the working solution.
    pub fn working_solution(&self) -> &S {
        self.inner.working_solution()
    }

    /// Returns a mutable reference to the working solution.
    pub fn working_solution_mut(&mut self) -> &mut S {
        self.inner.working_solution_mut()
    }

    /// Calculates and returns the current score.
    pub fn calculate_score(&mut self) -> S::Score {
        self.inner.calculate_score()
    }

    /// Returns the solution descriptor.
    pub fn solution_descriptor(&self) -> &SolutionDescriptor {
        self.inner.solution_descriptor()
    }

    /// Clones the working solution.
    pub fn clone_working_solution(&self) -> S {
        self.inner.clone_working_solution()
    }

    /// Called before a basic variable is changed.
    pub fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner
            .before_variable_changed(descriptor_index, entity_index);
    }

    /// Called after a basic variable is changed.
    pub fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner
            .after_variable_changed(descriptor_index, entity_index);
    }

    /// Retracts an element from constraint scoring.
    ///
    /// This is a low-level primitive - does NOT handle shadow variables.
    pub fn retract_element(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        element_idx: usize,
    ) {
        self.inner
            .retract_element(descriptor_index, entity_index, element_idx);
    }

    /// Inserts an element into constraint scoring.
    ///
    /// This is a low-level primitive - does NOT handle shadow variables.
    pub fn insert_element(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        element_idx: usize,
    ) {
        self.inner
            .insert_element(descriptor_index, entity_index, element_idx);
    }

    /// Tracks an element modification for undo.
    ///
    /// Call this after modifying an element so undo_changes knows what to restore.
    pub fn track_element_change(
        &mut self,
        descriptor_index: usize,
        entity_index: usize,
        position: usize,
        element_idx: usize,
    ) {
        let key = (descriptor_index, entity_index, position, element_idx);
        if !self.modified_elements.contains(&key) {
            self.modified_elements.push(key);
        }
    }

    /// Registers an undo closure.
    pub fn register_undo(&mut self, undo: Box<dyn FnOnce(&mut S) + Send>) {
        self.undo_stack.push(undo);
    }

    /// Undoes all recorded changes in reverse order.
    ///
    /// For incremental scoring correctness:
    /// 1. Retract current (post-move) contributions from each modified element
    /// 2. Run undo closures to restore planning variable values
    /// 3. Insert restored contributions
    ///
    /// Note: Shadow variable handling is done by the caller (proc-macro-generated code).
    pub fn undo_changes(&mut self) {
        // Step 1: Retract current contributions before restoring values
        for &(descriptor_idx, entity_idx, _position, element_idx) in &self.modified_elements {
            self.inner
                .retract_element(descriptor_idx, entity_idx, element_idx);
        }

        // Step 2: Process undo closures in reverse order
        while let Some(undo) = self.undo_stack.pop() {
            undo(self.inner.working_solution_mut());
        }

        // Step 3: Insert restored contributions
        for (descriptor_idx, entity_idx, _position, element_idx) in self.modified_elements.drain(..)
        {
            self.inner
                .insert_element(descriptor_idx, entity_idx, element_idx);
        }
    }

    /// Resets the recording state for reuse.
    ///
    /// Call this at the start of each step to reuse the Vec allocations.
    pub fn reset(&mut self) {
        self.undo_stack.clear();
        self.modified_elements.clear();
    }

    /// Returns the number of recorded undo closures.
    pub fn change_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns true if there are no recorded changes.
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }

    /// Returns the number of entities for a given descriptor index.
    pub fn entity_count(&self, descriptor_index: usize) -> usize {
        self.inner.entity_count(descriptor_index)
    }

    /// Returns true - incremental scoring is supported.
    pub fn is_incremental(&self) -> bool {
        self.inner.is_incremental()
    }

    /// Returns the cached score.
    pub fn get_score(&self) -> S::Score {
        self.inner.get_score()
    }

    /// Returns a mutable reference to the inner score director.
    ///
    /// Use this to pass the score director to move's `do_move` method.
    pub fn inner_mut(&mut self) -> &mut ScoreDirector<S, C> {
        self.inner
    }

    /// Returns a reference to the inner score director.
    pub fn inner(&self) -> &ScoreDirector<S, C> {
        self.inner
    }
}
