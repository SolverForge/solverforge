// Tests for RecordingDirector.

use crate::director::recording::RecordingDirector;
use crate::director::score_director::ScoreDirector;
use crate::Director;
use solverforge_core::score::SoftScore;
use solverforge_test::nqueens::{
    create_nqueens_descriptor, get_row, set_row, NQueensSolution, Queen,
};

fn create_inner(queens: Vec<Queen>) -> ScoreDirector<NQueensSolution, ()> {
    let descriptor = create_nqueens_descriptor();
    ScoreDirector::simple(NQueensSolution::new(queens), descriptor, |s, _| {
        s.queens.len()
    })
}

#[test]
fn test_recording_register_undo() {
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 0)]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

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
    let mut inner = create_inner(vec![
        Queen::assigned(0, 0, 0),
        Queen::assigned(1, 1, 1),
        Queen::assigned(2, 2, 2),
    ]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

        // Change multiple entities, registering typed undo for each
        for i in 0..3 {
            let old = get_row(recording.working_solution(), i);
            set_row(recording.working_solution_mut(), i, Some(10 + i as i64));
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
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 0)]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

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
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 0)]);

    let mut recording = RecordingDirector::new(&mut inner);

    recording.register_undo(Box::new(|_: &mut NQueensSolution| {}));
    assert_eq!(recording.change_count(), 1);

    recording.reset();
    assert!(recording.is_empty());
}

#[test]
fn test_recording_calculate_score() {
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 0), Queen::assigned(1, 1, 1)]);

    let mut recording = RecordingDirector::new(&mut inner);

    // Score is zero (empty constraint set)
    let score1 = recording.calculate_score();
    assert_eq!(score1, SoftScore::of(0));

    // Change a queen row and verify score is still computable
    let old = get_row(recording.working_solution(), 1);
    set_row(recording.working_solution_mut(), 1, Some(3));
    recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
        set_row(s, 1, old);
    }));

    let score2 = recording.calculate_score();
    assert_eq!(score2, SoftScore::of(0));

    // Undo and recalculate
    recording.undo_changes();
    let score3 = recording.calculate_score();
    assert_eq!(score3, SoftScore::of(0));
}

#[test]
fn test_recording_undo_none_to_some() {
    let mut inner = create_inner(vec![Queen::unassigned(0, 0)]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

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
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 5)]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

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
    let mut inner = create_inner(vec![]);

    let recording = RecordingDirector::new(&mut inner);
    assert!(recording.is_incremental()); // ScoreDirector is incremental
}

#[test]
fn test_recording_entity_count() {
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 0), Queen::assigned(1, 1, 1)]);

    let recording = RecordingDirector::new(&mut inner);
    assert_eq!(recording.entity_count(0), Some(2));
    assert_eq!(recording.total_entity_count(), Some(2));
}
