//! Tests for RecordingScoreDirector.

use super::recording::RecordingScoreDirector;
use super::ScoreDirector;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::SimpleScore;

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

fn get_row(s: &NQueensSolution, idx: usize) -> Option<i32> {
    s.queens.get(idx).and_then(|q| q.row)
}

fn set_row(s: &mut NQueensSolution, idx: usize, v: Option<i32>) {
    if let Some(q) = s.queens.get_mut(idx) {
        q.row = v;
    }
}

/// Creates a test director with empty constraints.
/// For RecordingScoreDirector tests, we only care about undo behavior,
/// not actual scoring.
fn create_test_director(solution: NQueensSolution) -> ScoreDirector<NQueensSolution, ()> {
    ScoreDirector::new(solution, ())
}

#[test]
fn test_recording_register_undo() {
    let solution = NQueensSolution {
        queens: vec![Queen {
            id: 0,
            row: Some(0),
        }],
        score: None,
    };

    let mut inner = create_test_director(solution);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

        let old_value = get_row(recording.working_solution(), 0);

        set_row(recording.working_solution_mut(), 0, Some(5));

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
            Queen {
                id: 0,
                row: Some(0),
            },
            Queen {
                id: 1,
                row: Some(1),
            },
            Queen {
                id: 2,
                row: Some(2),
            },
        ],
        score: None,
    };

    let mut inner = create_test_director(solution);

    {
        let mut recording = RecordingScoreDirector::new(&mut inner);

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
        queens: vec![Queen {
            id: 0,
            row: Some(0),
        }],
        score: None,
    };

    let mut inner = create_test_director(solution);

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
        queens: vec![Queen {
            id: 0,
            row: Some(0),
        }],
        score: None,
    };

    let mut inner = create_test_director(solution);

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
            Queen {
                id: 0,
                row: Some(0),
            },
            Queen {
                id: 1,
                row: Some(1),
            },
        ],
        score: None,
    };

    let mut inner = create_test_director(solution);

    let mut recording = RecordingScoreDirector::new(&mut inner);

    // With empty constraints, score is always 0
    let score1 = recording.calculate_score();
    assert_eq!(score1, SimpleScore::of(0));

    // Change value
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
    assert_eq!(score3, SimpleScore::of(0));
}

#[test]
fn test_recording_undo_none_to_some() {
    let solution = NQueensSolution {
        queens: vec![Queen { id: 0, row: None }],
        score: None,
    };

    let mut inner = create_test_director(solution);

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
        queens: vec![Queen {
            id: 0,
            row: Some(5),
        }],
        score: None,
    };

    let mut inner = create_test_director(solution);

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

    let mut inner = create_test_director(solution);

    let recording = RecordingScoreDirector::new(&mut inner);
    assert!(recording.is_incremental()); // ScoreDirector IS incremental
}

#[test]
fn test_recording_entity_count() {
    let solution = NQueensSolution {
        queens: vec![
            Queen {
                id: 0,
                row: Some(0),
            },
            Queen {
                id: 1,
                row: Some(1),
            },
        ],
        score: None,
    };

    let mut inner = create_test_director(solution);

    let recording = RecordingScoreDirector::new(&mut inner);
    assert_eq!(recording.entity_count(0), 0); // No entity counter configured
}
