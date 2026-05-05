/* Recording score director for automatic undo tracking.

The `RecordingDirector` wraps an existing score director and stores
typed undo closures registered by moves. This enables zero-erasure undo:

```text
// Pattern:
let mut recording = RecordingDirector::new(&mut inner_sd);
move.do_move(&mut recording);  // Move registers typed undo closure
let score = recording.calculate_score();
recording.undo_changes();  // Calls undo closures in reverse order
```

Moves capture old values using typed getters and register undo closures
via `register_undo()`. No BoxedValue, no type erasure on the undo path.
*/

use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};

use crate::api::constraint_set::ConstraintMetadata;

use super::{Director, DirectorScoreState};

/* A score director wrapper that stores typed undo closures.

Moves register their own typed undo closures via `register_undo()`.
This enables zero-erasure undo - no BoxedValue, no downcasting.

# Example

```
use solverforge_scoring::director::{RecordingDirector, Director};
use solverforge_scoring::director::score_director::ScoreDirector;
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use std::any::TypeId;

#[derive(Clone)]
struct Solution { value: i32, score: Option<SoftScore> }

impl PlanningSolution for Solution {
type Score = SoftScore;
fn score(&self) -> Option<Self::Score> { self.score }
fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
}

let mut sd = ScoreDirector::simple(
Solution { value: 10, score: None },
SolutionDescriptor::new("Solution", TypeId::of::<Solution>()),
|_, _| 0,
);

// Wrap in recording director
let mut recording = RecordingDirector::new(&mut sd);

// Make a change and register undo
let old_value = recording.working_solution().value;
recording.working_solution_mut().value = 20;
recording.register_undo(Box::new(move |s| s.value = old_value));

assert_eq!(recording.working_solution().value, 20);

// Undo restores the original value
recording.undo_changes();
assert_eq!(recording.working_solution().value, 10);
```
*/
pub struct RecordingDirector<'a, S: PlanningSolution, D: Director<S>> {
    inner: &'a mut D,
    // Typed undo closures registered by moves.
    undo_stack: Vec<Box<dyn FnOnce(&mut S) + Send>>,
    // Entities modified during this step that need shadow refresh after undo.
    // Stores (descriptor_index, entity_index) pairs.
    modified_entities: Vec<(usize, usize)>,
    initial_score_state: Option<DirectorScoreState<S::Score>>,
}

impl<'a, S: PlanningSolution, D: Director<S>> RecordingDirector<'a, S, D> {
    // Creates a new recording score director wrapping the inner director.
    pub fn new(inner: &'a mut D) -> Self {
        Self {
            initial_score_state: Some(inner.snapshot_score_state()),
            inner,
            undo_stack: Vec::with_capacity(16),
            modified_entities: Vec::with_capacity(8),
        }
    }

    /* Undoes all recorded changes in reverse order.

    For incremental scoring correctness:
    1. Retract current (post-move) contributions from each modified entity
    2. Run undo closures to restore planning variable values
    3. Update shadows and insert restored contributions
    */
    pub fn undo_changes(&mut self) {
        // Step 1: Retract current contributions before restoring values
        for &(descriptor_idx, entity_idx) in &self.modified_entities {
            self.inner
                .before_variable_changed(descriptor_idx, entity_idx);
        }

        // Step 2: Process undo closures in reverse order
        while let Some(undo) = self.undo_stack.pop() {
            undo(self.inner.working_solution_mut());
        }

        // Step 3: Update shadows and insert restored contributions
        for (descriptor_idx, entity_idx) in self.modified_entities.drain(..) {
            self.inner
                .after_variable_changed(descriptor_idx, entity_idx);
        }

        if let Some(initial_score_state) = self.initial_score_state.take() {
            self.inner.restore_score_state(initial_score_state);
        }
        self.initial_score_state = Some(self.inner.snapshot_score_state());
    }

    /* Resets the recording state for reuse.

    Call this at the start of each step to reuse the Vec allocations.
    */
    pub fn reset(&mut self) {
        self.undo_stack.clear();
        self.modified_entities.clear();
        self.initial_score_state = Some(self.inner.snapshot_score_state());
    }

    // Returns the number of recorded undo closures.
    pub fn change_count(&self) -> usize {
        self.undo_stack.len()
    }

    // Returns true if there are no recorded changes.
    pub fn is_empty(&self) -> bool {
        self.undo_stack.is_empty()
    }
}

impl<S: PlanningSolution, D: Director<S>> Director<S> for RecordingDirector<'_, S, D> {
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

    fn before_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner
            .before_variable_changed(descriptor_index, entity_index);
    }

    fn after_variable_changed(&mut self, descriptor_index: usize, entity_index: usize) {
        self.inner
            .after_variable_changed(descriptor_index, entity_index);

        // Track entity for post-undo shadow refresh (avoid duplicates)
        let key = (descriptor_index, entity_index);
        if !self.modified_entities.contains(&key) {
            self.modified_entities.push(key);
        }
    }

    fn entity_count(&self, descriptor_index: usize) -> Option<usize> {
        self.inner.entity_count(descriptor_index)
    }

    fn total_entity_count(&self) -> Option<usize> {
        self.inner.total_entity_count()
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata<'_>> {
        self.inner.constraint_metadata()
    }

    fn is_incremental(&self) -> bool {
        self.inner.is_incremental()
    }

    fn snapshot_score_state(&self) -> DirectorScoreState<S::Score> {
        self.inner.snapshot_score_state()
    }

    fn restore_score_state(&mut self, state: DirectorScoreState<S::Score>) {
        self.inner.restore_score_state(state);
    }

    fn reset(&mut self) {
        // Forward to inner
        self.inner.reset();
        // Also clear our recording state
        self.undo_stack.clear();
        self.modified_entities.clear();
        self.initial_score_state = Some(self.inner.snapshot_score_state());
    }

    fn register_undo(&mut self, undo: Box<dyn FnOnce(&mut S) + Send>) {
        self.undo_stack.push(undo);
    }
}
