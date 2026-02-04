//! Tests for score director functionality.

use super::*;

#[test]
fn test_recording_score_director() {
    // Test that RecordingScoreDirector works correctly with DynamicSolution
    use crate::moves::DynamicChangeMove;
    use solverforge_scoring::director::typed::TypedScoreDirector;
    use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};
    use solverforge_solver::heuristic::r#move::Move;

    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

    let mut solution = DynamicSolution::new(desc.clone());
    // Construction result: rows [0, 2, 0, 1] (score -2)
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(2)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(2, vec![DynamicValue::I64(2), DynamicValue::I64(0)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(3, vec![DynamicValue::I64(3), DynamicValue::I64(1)]),
    );

    let mut constraints = DynamicConstraintSet::new();
    constraints.add(make_row_conflict_constraint(&desc));
    constraints.add(make_asc_diagonal_constraint(&desc));
    constraints.add(make_desc_diagonal_constraint(&desc));

    // Use TypedScoreDirector like the real solver does
    let descriptor = solverforge_core::domain::SolutionDescriptor::new(
        "DynamicSolution",
        std::any::TypeId::of::<DynamicSolution>(),
    );
    fn entity_counter(s: &DynamicSolution, idx: usize) -> usize {
        s.entities.get(idx).map(|v| v.len()).unwrap_or(0)
    }
    let mut director =
        TypedScoreDirector::with_descriptor(solution, constraints, descriptor, entity_counter);

    // Calculate initial score
    let initial_score = director.calculate_score();
    eprintln!("Initial score: {:?}", initial_score);
    assert_eq!(initial_score, HardSoftScore::of_hard(-2));

    // Create the improving move: Queen 2 row 0 -> 3
    let improving_move = DynamicChangeMove::new(0, 2, 1, "row", DynamicValue::I64(3));

    // Use RecordingScoreDirector to evaluate the move
    let move_score = {
        let mut recording = RecordingScoreDirector::new(&mut director);

        // Execute move
        improving_move.do_move(&mut recording);

        // Calculate resulting score
        let score = recording.calculate_score();
        eprintln!("Score after move: {:?}", score);

        // Undo the move
        recording.undo_changes();

        score
    };

    // Verify the move improves the score
    // Move: Queen 2 row 0->3, with queens at rows [0,2,3,1]
    // - No row conflicts (all different)
    // - Ascending diagonal: Q1 (col=1, row=2) and Q2 (col=2, row=3) both have row-col=1
    // Final score: -1hard (one diagonal conflict remains)
    assert_eq!(
        move_score,
        HardSoftScore::of_hard(-1),
        "Move should resolve row conflict but still have diagonal conflict"
    );
    assert!(move_score > initial_score, "Move should improve score");

    // Verify undo worked - score should be back to initial
    let restored_score = director.calculate_score();
    eprintln!("Score after undo: {:?}", restored_score);
    assert_eq!(
        restored_score, initial_score,
        "Score should be restored after undo"
    );

    // Verify entity is restored
    let entity = director.working_solution().get_entity(0, 2).unwrap();
    assert_eq!(
        entity.fields[1],
        DynamicValue::I64(0),
        "Entity should be restored"
    );
}

#[test]
fn test_score_comparison() {
    // Verify that -1hard > -2hard (better score)
    let score_minus_2 = HardSoftScore::of_hard(-2);
    let score_minus_1 = HardSoftScore::of_hard(-1);
    assert!(
        score_minus_1 > score_minus_2,
        "-1hard should be better than -2hard"
    );
    eprintln!("-1hard > -2hard: {}", score_minus_1 > score_minus_2);
}

#[test]
fn test_constraint_evaluation() {
    // Test with a known valid 4-queens solution: rows [1, 3, 0, 2]
    // Queen 0 at (0, 1), Queen 1 at (1, 3), Queen 2 at (2, 0), Queen 3 at (3, 2)
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, 4));

    let mut solution = DynamicSolution::new(desc.clone());
    // Valid 4-queens: rows [1, 3, 0, 2]
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(1)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(1, vec![DynamicValue::I64(1), DynamicValue::I64(3)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(2, vec![DynamicValue::I64(2), DynamicValue::I64(0)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(3, vec![DynamicValue::I64(3), DynamicValue::I64(2)]),
    );

    let mut constraints = DynamicConstraintSet::new();
    constraints.add(make_row_conflict_constraint(&desc));
    constraints.add(make_asc_diagonal_constraint(&desc));
    constraints.add(make_desc_diagonal_constraint(&desc));

    // Must initialize for incremental scoring
    let score = constraints.initialize_all(&solution);
    eprintln!("Valid 4-queens score: {:?}", score);
    assert_eq!(
        score,
        HardSoftScore::ZERO,
        "Valid 4-queens should have score 0"
    );
}
