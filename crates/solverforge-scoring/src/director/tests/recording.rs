// Tests for RecordingDirector.

use crate::director::recording::RecordingDirector;
use crate::director::score_director::ScoreDirector;
use crate::Director;
use crate::{ConstraintAnalysis, ConstraintMetadata, ConstraintResult, ConstraintSet};
use solverforge_core::domain::{PlanningSolution, SolutionDescriptor};
use solverforge_core::score::SoftScore;
use solverforge_core::ConstraintRef;
use solverforge_test::nqueens::{
    create_nqueens_descriptor, get_queen_row, set_queen_row, NQueensSolution, Queen,
};
use std::any::TypeId;

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
        let old_value = get_queen_row(recording.working_solution(), 0, 0);

        // Apply change using typed setter
        set_queen_row(recording.working_solution_mut(), 0, 0, Some(5));

        // Register typed undo closure
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_queen_row(s, 0, 0, old_value);
        }));

        assert_eq!(recording.change_count(), 1);

        // Verify change was applied
        assert_eq!(get_queen_row(recording.working_solution(), 0, 0), Some(5));

        // Undo
        recording.undo_changes();
        assert!(recording.is_empty());
    }

    // Verify original value restored
    assert_eq!(get_queen_row(inner.working_solution(), 0, 0), Some(0));
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
            let old = get_queen_row(recording.working_solution(), i, 0);
            set_queen_row(recording.working_solution_mut(), i, 0, Some(10 + i as i64));
            recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
                set_queen_row(s, i, 0, old);
            }));
        }

        assert_eq!(recording.change_count(), 3);

        // Undo all
        recording.undo_changes();
    }

    // Verify all restored
    assert_eq!(get_queen_row(inner.working_solution(), 0, 0), Some(0));
    assert_eq!(get_queen_row(inner.working_solution(), 1, 0), Some(1));
    assert_eq!(get_queen_row(inner.working_solution(), 2, 0), Some(2));
}

#[test]
fn test_recording_undo_same_entity_twice() {
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 0)]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

        // First change: 0 -> 5
        let old1 = get_queen_row(recording.working_solution(), 0, 0);
        set_queen_row(recording.working_solution_mut(), 0, 0, Some(5));
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_queen_row(s, 0, 0, old1);
        }));

        // Second change: 5 -> 10
        let old2 = get_queen_row(recording.working_solution(), 0, 0);
        set_queen_row(recording.working_solution_mut(), 0, 0, Some(10));
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_queen_row(s, 0, 0, old2);
        }));

        assert_eq!(recording.change_count(), 2);

        // Undo all - should restore to original 0
        recording.undo_changes();
    }

    assert_eq!(get_queen_row(inner.working_solution(), 0, 0), Some(0));
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
    let old = get_queen_row(recording.working_solution(), 1, 0);
    set_queen_row(recording.working_solution_mut(), 1, 0, Some(3));
    recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
        set_queen_row(s, 1, 0, old);
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
        let old = get_queen_row(recording.working_solution(), 0, 0);
        set_queen_row(recording.working_solution_mut(), 0, 0, Some(5));
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_queen_row(s, 0, 0, old);
        }));

        // Undo
        recording.undo_changes();
    }

    // Should be None again
    assert_eq!(get_queen_row(inner.working_solution(), 0, 0), None);
}

#[test]
fn test_recording_undo_some_to_none() {
    let mut inner = create_inner(vec![Queen::assigned(0, 0, 5)]);

    {
        let mut recording = RecordingDirector::new(&mut inner);

        // Set from Some to None
        let old = get_queen_row(recording.working_solution(), 0, 0);
        set_queen_row(recording.working_solution_mut(), 0, 0, None);
        recording.register_undo(Box::new(move |s: &mut NQueensSolution| {
            set_queen_row(s, 0, 0, old);
        }));

        // Undo
        recording.undo_changes();
    }

    // Should be Some(5) again
    assert_eq!(get_queen_row(inner.working_solution(), 0, 0), Some(5));
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

#[test]
fn test_recording_preserves_constraint_metadata() {
    let descriptor =
        SolutionDescriptor::new("ScoreProbeSolution", TypeId::of::<ScoreProbeSolution>());
    let mut inner = ScoreDirector::with_descriptor(
        ScoreProbeSolution {
            value: 2,
            score: None,
        },
        ValueConstraintSet,
        descriptor,
        |_, descriptor_index| usize::from(descriptor_index == 0),
    );

    let recording = RecordingDirector::new(&mut inner);

    assert_eq!(recording.constraint_metadata().len(), 1);
    assert_eq!(recording.constraint_metadata()[0].name(), "value");
    assert_eq!(
        recording.constraint_is_hard(&ConstraintRef::new("", "value")),
        Some(false)
    );
}

#[derive(Clone)]
struct ScoreProbeSolution {
    value: i64,
    score: Option<SoftScore>,
}

#[derive(Default)]
struct ValueConstraintSet;

impl ConstraintSet<ScoreProbeSolution, SoftScore> for ValueConstraintSet {
    fn evaluate_all(&self, solution: &ScoreProbeSolution) -> SoftScore {
        SoftScore::of(solution.value)
    }

    fn constraint_count(&self) -> usize {
        1
    }

    fn constraint_metadata(&self) -> Vec<ConstraintMetadata> {
        vec![ConstraintMetadata::new(
            ConstraintRef::new("", "value"),
            false,
        )]
    }

    fn evaluate_each(&self, solution: &ScoreProbeSolution) -> Vec<ConstraintResult<SoftScore>> {
        vec![ConstraintResult {
            name: "value".to_string(),
            score: self.evaluate_all(solution),
            match_count: 1,
            is_hard: false,
        }]
    }

    fn evaluate_detailed(
        &self,
        solution: &ScoreProbeSolution,
    ) -> Vec<ConstraintAnalysis<SoftScore>> {
        vec![ConstraintAnalysis::new(
            ConstraintRef::new("", "value"),
            SoftScore::of(1),
            self.evaluate_all(solution),
            Vec::new(),
            false,
        )]
    }

    fn initialize_all(&mut self, solution: &ScoreProbeSolution) -> SoftScore {
        self.evaluate_all(solution)
    }

    fn on_insert_all(
        &mut self,
        solution: &ScoreProbeSolution,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        self.evaluate_all(solution)
    }

    fn on_retract_all(
        &mut self,
        solution: &ScoreProbeSolution,
        _entity_index: usize,
        _descriptor_index: usize,
    ) -> SoftScore {
        -self.evaluate_all(solution)
    }

    fn reset_all(&mut self) {}
}

impl PlanningSolution for ScoreProbeSolution {
    type Score = SoftScore;

    fn score(&self) -> Option<Self::Score> {
        self.score
    }

    fn set_score(&mut self, score: Option<Self::Score>) {
        self.score = score;
    }
}

#[test]
fn test_recording_undo_restores_committed_cached_score_state() {
    let descriptor =
        SolutionDescriptor::new("ScoreProbeSolution", TypeId::of::<ScoreProbeSolution>());
    let mut inner = ScoreDirector::with_descriptor(
        ScoreProbeSolution {
            value: 2,
            score: None,
        },
        ValueConstraintSet,
        descriptor,
        |_, descriptor_index| usize::from(descriptor_index == 0),
    );

    assert_eq!(inner.calculate_score(), SoftScore::of(2));

    {
        let mut recording = RecordingDirector::new(&mut inner);
        let old_value = recording.working_solution().value;
        recording.before_variable_changed(0, 0);
        recording.working_solution_mut().value = 7;
        recording.after_variable_changed(0, 0);
        recording.register_undo(Box::new(move |solution: &mut ScoreProbeSolution| {
            solution.value = old_value;
        }));

        assert_eq!(recording.calculate_score(), SoftScore::of(7));
        recording.undo_changes();
    }

    assert_eq!(inner.working_solution().value, 2);
    assert_eq!(inner.working_solution().score(), Some(SoftScore::of(2)));
    assert_eq!(
        inner.clone_working_solution().score(),
        Some(SoftScore::of(2))
    );
    assert_eq!(inner.calculate_score(), SoftScore::of(2));
}
