//! Tests for mimic selectors.

use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor, TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
use std::any::TypeId;

use super::EntityReference;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::mimic::{
    MimicRecorder, MimicRecordingEntitySelector, MimicReplayingEntitySelector,
};
use crate::heuristic::selector::EntitySelector;

#[derive(Clone, Debug)]
struct Queen {
    id: i64,
}

#[derive(Clone, Debug)]
struct NQueensSolution {
    queens: Vec<Queen>,
    score: Option<SimpleScore>,
}

impl PlanningSolution for NQueensSolution {
    type Score = SimpleScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

fn create_test_director(
    n: usize,
) -> SimpleScoreDirector<NQueensSolution, impl Fn(&NQueensSolution) -> SimpleScore> {
    let queens: Vec<_> = (0..n).map(|i| Queen { id: i as i64 }).collect();

    let solution = NQueensSolution {
        queens,
        score: None,
    };

    let extractor = Box::new(TypedEntityExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));
    let entity_desc =
        EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens").with_extractor(extractor);

    let descriptor = SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc);

    SimpleScoreDirector::with_calculator(solution, descriptor, |_| SimpleScore::of(0))
}

#[test]
fn test_mimic_recording_selector() {
    let director = create_test_director(3);

    // Verify entity IDs
    let solution = director.working_solution();
    for (i, queen) in solution.queens.iter().enumerate() {
        assert_eq!(queen.id, i as i64);
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
    let director = create_test_director(3);

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
    let director = create_test_director(3);

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
    let director = create_test_director(0);

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
