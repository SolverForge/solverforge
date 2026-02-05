//! Mimic selectors for synchronized selection across multiple selectors.
//!
//! Mimic selectors enable multiple selectors to select the same element in lockstep.
//! This is essential for:
//! - Nearby selection: Get the "origin" entity that was already selected
//! - Coordinated moves: Ensure multiple parts of a move reference the same entity
//!
//! # Architecture
//!
//! - [`MimicRecordingEntitySelector`]: Wraps a child selector and records each selected entity
//! - [`MimicReplayingEntitySelector`]: Replays the entity recorded by a recording selector

use std::fmt::Debug;
use std::sync::{Arc, RwLock};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::entity::{EntityReference, EntitySelector};

/// Shared state between recording and replaying selectors.
#[derive(Debug, Default)]
struct MimicState {
    /// Whether hasNext has been called on the recorder.
    has_next_recorded: bool,
    /// The result of the last hasNext call.
    has_next: bool,
    /// Whether next has been called on the recorder.
    next_recorded: bool,
    /// The last recorded entity reference.
    recorded_entity: Option<EntityReference>,
}

/// Handle for sharing mimic state between recording and replaying selectors.
#[derive(Debug, Clone)]
pub struct MimicRecorder {
    state: Arc<RwLock<MimicState>>,
    /// Identifier for debugging.
    id: String,
}

impl MimicRecorder {
    /// Creates a new mimic recorder with the given identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            state: Arc::new(RwLock::new(MimicState::default())),
            id: id.into(),
        }
    }

    /// Records a has_next result.
    fn record_has_next(&self, has_next: bool) {
        let mut state = self.state.write().unwrap();
        state.has_next_recorded = true;
        state.has_next = has_next;
        state.next_recorded = false;
        state.recorded_entity = None;
    }

    /// Records a next result.
    fn record_next(&self, entity: EntityReference) {
        let mut state = self.state.write().unwrap();
        state.has_next_recorded = true;
        state.has_next = true;
        state.next_recorded = true;
        state.recorded_entity = Some(entity);
    }

    /// Gets the recorded has_next state.
    pub fn get_has_next(&self) -> Option<bool> {
        let state = self.state.read().unwrap();
        if state.has_next_recorded {
            Some(state.has_next)
        } else {
            None
        }
    }

    /// Gets the recorded entity.
    pub fn get_recorded_entity(&self) -> Option<EntityReference> {
        let state = self.state.read().unwrap();
        if state.next_recorded {
            state.recorded_entity
        } else {
            None
        }
    }

    /// Returns the ID of this recorder.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Resets the state for a new iteration.
    pub fn reset(&self) {
        *self.state.write().unwrap() = MimicState::default();
    }
}

/// An entity selector that records each selected entity for replay by other selectors.
///
/// This is used to synchronize selection across multiple selectors. The recording
/// selector wraps a child selector and broadcasts each selected entity to all
/// replaying selectors that share the same recorder.
///
/// # Zero-Erasure Design
///
/// The child entity selector `ES` is stored as a concrete generic type parameter,
/// eliminating virtual dispatch overhead when iterating over entities.
pub struct MimicRecordingEntitySelector<S, ES> {
    /// The child selector that actually selects entities (zero-erasure).
    child: ES,
    /// The recorder that broadcasts selections.
    recorder: MimicRecorder,
    /// Marker for solution type.
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S, ES> MimicRecordingEntitySelector<S, ES> {
    /// Creates a new recording selector wrapping the given child selector.
    pub fn new(child: ES, recorder: MimicRecorder) -> Self {
        Self {
            child,
            recorder,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Returns the recorder for creating replaying selectors.
    pub fn recorder(&self) -> MimicRecorder {
        self.recorder.clone()
    }
}

impl<S: PlanningSolution, ES: Debug> Debug for MimicRecordingEntitySelector<S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MimicRecordingEntitySelector")
            .field("child", &self.child)
            .field("recorder_id", &self.recorder.id)
            .finish()
    }
}

impl<S, ES> EntitySelector<S> for MimicRecordingEntitySelector<S, ES>
where
    S: PlanningSolution,
    ES: EntitySelector<S>,
{
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        score_director: &'a D,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a> {
        // Reset for new iteration
        self.recorder.reset();

        let child_iter = self.child.iter(score_director);
        Box::new(RecordingIterator {
            inner: child_iter,
            recorder: &self.recorder,
        })
    }

    fn size<D: ScoreDirector<S>>(&self, score_director: &D) -> usize {
        self.child.size(score_director)
    }

    fn is_never_ending(&self) -> bool {
        self.child.is_never_ending()
    }
}

/// Iterator that records each entity as it's yielded.
struct RecordingIterator<'a> {
    inner: Box<dyn Iterator<Item = EntityReference> + 'a>,
    recorder: &'a MimicRecorder,
}

impl<'a> Iterator for RecordingIterator<'a> {
    type Item = EntityReference;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.inner.next();
        match next {
            Some(entity) => {
                self.recorder.record_next(entity);
                Some(entity)
            }
            None => {
                self.recorder.record_has_next(false);
                None
            }
        }
    }
}

/// An entity selector that replays the last entity recorded by a recording selector.
///
/// This selector always yields exactly one entity (the last one recorded) or no entities
/// if the recording selector hasn't recorded anything yet.
pub struct MimicReplayingEntitySelector {
    /// The recorder to replay from.
    recorder: MimicRecorder,
}

impl MimicReplayingEntitySelector {
    /// Creates a new replaying selector that replays from the given recorder.
    pub fn new(recorder: MimicRecorder) -> Self {
        Self { recorder }
    }
}

impl Debug for MimicReplayingEntitySelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MimicReplayingEntitySelector")
            .field("recorder_id", &self.recorder.id)
            .finish()
    }
}

impl<S: PlanningSolution> EntitySelector<S> for MimicReplayingEntitySelector {
    fn iter<'a, D: ScoreDirector<S>>(
        &'a self,
        _score_director: &'a D,
    ) -> Box<dyn Iterator<Item = EntityReference> + 'a> {
        Box::new(ReplayingIterator {
            recorder: &self.recorder,
            returned: false,
        })
    }

    fn size<D: ScoreDirector<S>>(&self, _score_director: &D) -> usize {
        // At most one entity is returned
        if self.recorder.get_recorded_entity().is_some() {
            1
        } else {
            0
        }
    }

    fn is_never_ending(&self) -> bool {
        false
    }
}

/// Iterator that replays a single recorded entity.
struct ReplayingIterator<'a> {
    recorder: &'a MimicRecorder,
    returned: bool,
}

impl<'a> Iterator for ReplayingIterator<'a> {
    type Item = EntityReference;

    fn next(&mut self) -> Option<Self::Item> {
        if self.returned {
            return None;
        }

        // Check if something was recorded
        match self.recorder.get_recorded_entity() {
            Some(entity) => {
                self.returned = true;
                Some(entity)
            }
            None => {
                // Check has_next to provide better error handling
                match self.recorder.get_has_next() {
                    Some(false) => None, // Recording selector exhausted
                    Some(true) => panic!(
                        "MimicReplayingEntitySelector: Recording selector's hasNext() was true \
                         but next() was never called. Ensure the recording selector's iterator \
                         is advanced before using the replaying selector."
                    ),
                    None => panic!(
                        "MimicReplayingEntitySelector: No recording found. \
                         The recording selector must be iterated before the replaying selector."
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heuristic::selector::entity::FromSolutionEntitySelector;
    use crate::test_utils::create_simple_nqueens_director;

    #[test]
    fn test_mimic_recording_selector() {
        let director = create_simple_nqueens_director(3);

        // Verify column values match indices
        let solution = director.working_solution();
        for (i, queen) in solution.queens.iter().enumerate() {
            assert_eq!(queen.column, i as i32);
        }

        let recorder = MimicRecorder::new("test");
        let child = FromSolutionEntitySelector::new(0);
        let recording = MimicRecordingEntitySelector::new(child, recorder);

        let entities: Vec<_> = recording.iter(&director).collect();
        assert_eq!(entities.len(), 3);
        assert_eq!(entities[0], EntityReference::new(0, 0));
        assert_eq!(entities[1], EntityReference::new(0, 1));
        assert_eq!(entities[2], EntityReference::new(0, 2));
    }

    #[test]
    fn test_mimic_replaying_selector() {
        let director = create_simple_nqueens_director(3);

        let recorder = MimicRecorder::new("test");
        let child = FromSolutionEntitySelector::new(0);
        let recording = MimicRecordingEntitySelector::new(child, recorder.clone());
        let replaying = MimicReplayingEntitySelector::new(recorder);

        // Iterate through recording selector
        let mut recording_iter = recording.iter(&director);

        // First entity recorded
        let first = recording_iter.next().unwrap();
        assert_eq!(first, EntityReference::new(0, 0));

        // Replaying should yield the same entity
        let replayed: Vec<_> = replaying.iter(&director).collect();
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0], EntityReference::new(0, 0));

        // Move to second entity
        let second = recording_iter.next().unwrap();
        assert_eq!(second, EntityReference::new(0, 1));

        // Replaying should now yield the second entity
        let replayed: Vec<_> = replaying.iter(&director).collect();
        assert_eq!(replayed.len(), 1);
        assert_eq!(replayed[0], EntityReference::new(0, 1));
    }

    #[test]
    fn test_mimic_synchronized_iteration() {
        let director = create_simple_nqueens_director(3);

        let recorder = MimicRecorder::new("test");
        let child = FromSolutionEntitySelector::new(0);
        let recording = MimicRecordingEntitySelector::new(child, recorder.clone());
        let replaying = MimicReplayingEntitySelector::new(recorder);

        // Simulate how this would be used in a move selector:
        // For each recorded entity, get the replayed entity
        for recorded in recording.iter(&director) {
            let replayed: Vec<_> = replaying.iter(&director).collect();
            assert_eq!(replayed.len(), 1);
            assert_eq!(replayed[0], recorded);
        }
    }

    #[test]
    fn test_mimic_empty_selector() {
        let director = create_simple_nqueens_director(0);

        let recorder = MimicRecorder::new("test");
        let child = FromSolutionEntitySelector::new(0);
        let recording = MimicRecordingEntitySelector::new(child, recorder.clone());
        let replaying = MimicReplayingEntitySelector::new(recorder);

        // Recording selector is empty
        let entities: Vec<_> = recording.iter(&director).collect();
        assert_eq!(entities.len(), 0);

        // Replaying should also be empty
        let replayed: Vec<_> = replaying.iter(&director).collect();
        assert_eq!(replayed.len(), 0);
    }
}
