//! Tests for RecordingScoreDirector.

use super::recording::RecordingScoreDirector;
use super::SimpleScoreDirector;
use crate::ScoreDirector;
use solverforge_core::domain::{
    EntityDescriptor, PlanningSolution, SolutionDescriptor,
    TypedEntityExtractor,
};
use solverforge_core::score::SimpleScore;
use std::any::TypeId;

#[derive(Clone, Debug, PartialEq)]
struct Queen {
    id: i64,
    row: Option<i32>,
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

// Typed getter - zero erasure
fn get_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
    s.queens.get(idx).and_then(|q| q.row)
}

// Typed setter - zero erasure
fn set_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
    if let Some(q) = s.queens.get_mut(idx) {
        q.row = v;
    }
}

fn get_queens(s: &NQueensSolution) -> &Vec<Queen> {
    &s.queens
}

fn get_queens_mut(s: &mut NQueensSolution) -> &mut Vec<Queen> {
    &mut s.queens
}

fn calculate_conflicts(solution: &NQueensSolution) -> SimpleScore {
    let mut conflicts = 0i64;
    let queens = &solution.queens;

    for i in 0..queens.len() {
        for j in (i + 1)..queens.len() {
            if let (Some(row_i), Some(row_j)) = (queens[i].row, queens[j].row) {
                if row_i == row_j {
                    conflicts += 1;
                }
                let col_diff = (j - i) as i32;
                if (row_i - row_j).abs() == col_diff {
                    conflicts += 1;
                }
            }
        }
    }

    SimpleScore::of(-conflicts)
}

fn create_test_descriptor() -> SolutionDescriptor {
    let extractor = Box::new(TypedEntityExtractor::new(
        "Queen",
        "queens",
        get_queens,
        get_queens_mut,
    ));let entity_desc = EntityDescriptor::new("Queen", TypeId::of::<Queen>(), "queens")
        .with_extractor(extractor);

    SolutionDescriptor::new("NQueensSolution", TypeId::of::<NQueensSolution>())
        .with_entity(entity_desc)
}

#[test]
fn test_recording_register_undo() {
    let solution = NQueensSolution {
        queens: vec![Queen { id: 0, row: Some(0) }],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

        // Capture old value using typed getter
        let old_value = get_row(recording.working_solution(), 0);

        // Apply change using typed setter
        set_row(recording.working_solution_mut(), 0, Some(5));

        // Register typed undo closure
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_row(s, 0, old_value);
        }));

        assert_eq!(recording.change_count(), 1);

        // Verify change was applied
        assert_eq!(get_row(recording.working_solution(), 0), Some(5));

        // Undo
        recording.undo_changes();
        assert!(recording.is_empty());
    }

    // Verify original value restored
    assert_eq!(get_row(inner.working_solution(), 0), Some(0));
}

#[test]
fn test_recording_multiple_undo() {
    let solution = NQueensSolution {
        queens: vec![
            Queen { id: 0, row: Some(0) },
            Queen { id: 1, row: Some(1) },
            Queen { id: 2, row: Some(2) },
        ],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

        // Change multiple entities, registering typed undo for each
        for i in 0..3 {
            let old = get_row(recording.working_solution(), i);
            set_row(recording.working_solution_mut(), i, Some(10 + i as i32));
            recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
                set_row(s, i, old);
            }));
        }

        assert_eq!(recording.change_count(), 3);

        // Undo all
        recording.undo_changes();
    }

    // Verify all restored
    assert_eq!(get_row(inner.working_solution(), 0), Some(0));
    assert_eq!(get_row(inner.working_solution(), 1), Some(1));
    assert_eq!(get_row(inner.working_solution(), 2), Some(2));
}

#[test]
fn test_recording_undo_same_entity_twice() {
    let solution = NQueensSolution {
        queens: vec![Queen { id: 0, row: Some(0) }],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

        // First change: 0 -> 5
        let old1 = get_row(recording.working_solution(), 0);
        set_row(recording.working_solution_mut(), 0, Some(5));
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_row(s, 0, old1);
        }));

        // Second change: 5 -> 10
        let old2 = get_row(recording.working_solution(), 0);
        set_row(recording.working_solution_mut(), 0, Some(10));
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_row(s, 0, old2);
        }));

        assert_eq!(recording.change_count(), 2);

        // Undo all - should restore to original 0
        recording.undo_changes();
    }

    assert_eq!(get_row(inner.working_solution(), 0), Some(0));
}

#[test]
fn test_recording_reset() {
    let solution = NQueensSolution {
        queens: vec![Queen { id: 0, row: Some(0) }],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    let mut recording = RecordingScoreDirector::new(&mut inner);

    recording.register_undo(Box::new(|_: &mut NQueensSolution| {}));
    assert_eq!(recording.change_count(), 1);

    recording.reset();
    assert!(recording.is_empty());
}

#[test]
fn test_recording_calculate_score() {
    let solution = NQueensSolution {
        queens: vec![
            Queen { id: 0, row: Some(0) },
            Queen { id: 1, row: Some(1) },
        ],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    let mut recording = RecordingScoreDirector::new(&mut inner);

    // Initial score (diagonal conflict)
    let score1 = recording.calculate_score();
    assert_eq!(score1, SimpleScore::of(-1));

    // Change to avoid conflict
    let old = get_row(recording.working_solution(), 1);
    set_row(recording.working_solution_mut(), 1, Some(3));
    recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
        set_row(s, 1, old);
    }));

    let score2 = recording.calculate_score();
    assert_eq!(score2, SimpleScore::of(0));

    // Undo and recalculate
    recording.undo_changes();
    let score3 = recording.calculate_score();
    assert_eq!(score3, SimpleScore::of(-1));
}

#[test]
fn test_recording_undo_none_to_some() {
    let solution = NQueensSolution {
        queens: vec![Queen { id: 0, row: None }],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

        // Set from None to Some
        let old = get_row(recording.working_solution(), 0);
        set_row(recording.working_solution_mut(), 0, Some(5));
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_row(s, 0, old);
        }));

        // Undo
        recording.undo_changes();
    }

    // Should be None again
    assert_eq!(get_row(inner.working_solution(), 0), None);
}

#[test]
fn test_recording_undo_some_to_none() {
    let solution = NQueensSolution {
        queens: vec![Queen { id: 0, row: Some(5) }],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

        // Set from Some to None
        let old = get_row(recording.working_solution(), 0);
        set_row(recording.working_solution_mut(), 0, None);
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_row(s, 0, old);
        }));

        // Undo
        recording.undo_changes();
    }

    // Should be Some(5) again
    assert_eq!(get_row(inner.working_solution(), 0), Some(5));
}

#[test]
fn test_recording_is_incremental() {
    let solution = NQueensSolution {
        queens: vec![],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    let recording = RecordingScoreDirector::new(&mut inner);
    assert!(!recording.is_incremental()); // SimpleScoreDirector is not incremental
}

#[test]
fn test_recording_entity_count() {
    let solution = NQueensSolution {
        queens: vec![
            Queen { id: 0, row: Some(0) },
            Queen { id: 1, row: Some(1) },
        ],
        score: None,
    };

    let descriptor = create_test_descriptor();
    let mut inner =
        SimpleScoreDirector::with_calculator(solution, descriptor, calculate_conflicts);

    let recording = RecordingScoreDirector::new(&mut inner);
    assert_eq!(recording.entity_count(0), Some(2));
    assert_eq!(recording.total_entity_count(), Some(2));
}
