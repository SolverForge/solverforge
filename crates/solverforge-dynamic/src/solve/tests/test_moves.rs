//! Tests for move application and evaluation.

use super::*;

#[test]
fn test_move_score_changes() {
    // Test that we can find improving moves from the construction solution
    // Construction results in: rows [0, 2, 0, 1] with score -2hard
    // Let's verify what scores we'd get from various moves

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
    // Construction result: rows [0, 2, 0, 1]
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

    // Must initialize for incremental scoring
    let initial_score = constraints.initialize_all(&solution);
    eprintln!("Initial score (rows [0,2,0,1]): {:?}", initial_score);

    // Now let's try each possible move using incremental scoring
    for queen_idx in 0..4 {
        let current_row = match &solution.get_entity(0, queen_idx).unwrap().fields[1] {
            DynamicValue::I64(r) => *r,
            _ => -1,
        };
        for new_row in 0i64..4 {
            if new_row == current_row {
                continue;
            }
            // Retract old value
            let delta1 = constraints.on_retract_all(&solution, queen_idx, 0);
            // Apply change
            solution.update_field(0, queen_idx, 1, DynamicValue::I64(new_row));
            // Insert new value
            let delta2 = constraints.on_insert_all(&solution, queen_idx, 0);
            let new_score = initial_score + delta1 + delta2;

            eprintln!(
                "  Queen {} move row {} -> {}: score {:?} (diff: {})",
                queen_idx,
                current_row,
                new_row,
                new_score,
                new_score.hard() - initial_score.hard()
            );

            // Undo: retract new, restore old, insert old
            constraints.on_retract_all(&solution, queen_idx, 0);
            solution.update_field(0, queen_idx, 1, DynamicValue::I64(current_row));
            constraints.on_insert_all(&solution, queen_idx, 0);
        }
    }
}

#[test]
fn test_move_application() {
    // Test that moves work correctly
    use crate::moves::DynamicChangeMove;
    use solverforge_core::domain::SolutionDescriptor;
    use solverforge_scoring::{ScoreDirector, SimpleScoreDirector};
    use solverforge_solver::heuristic::r#move::Move;
    use std::any::TypeId;

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
    // Start with Queen 0 at (0, 0)
    solution.add_entity(
        0,
        DynamicEntity::new(0, vec![DynamicValue::I64(0), DynamicValue::I64(0)]),
    );

    let sol_desc = SolutionDescriptor::new("DynamicSolution", TypeId::of::<DynamicSolution>());
    let mut director = SimpleScoreDirector::new(solution, sol_desc, |_: &DynamicSolution| {
        HardSoftScore::ZERO
    });

    // Create a move to change Queen 0's row from 0 to 2
    let change_move = DynamicChangeMove::new(0, 0, 1, "row", DynamicValue::I64(2));

    // Check move is doable
    assert!(change_move.is_doable(&director), "Move should be doable");

    // Apply the move
    change_move.do_move(&mut director);

    // Verify the change
    let sol = director.working_solution();
    let entity = sol.get_entity(0, 0).unwrap();
    eprintln!("After move: entity fields = {:?}", entity.fields);
    assert_eq!(
        entity.fields[1],
        DynamicValue::I64(2),
        "Row should be changed to 2"
    );
}

#[test]
fn test_local_search_simulation() {
    // Simulate what local search does: iterate moves, evaluate with RecordingScoreDirector
    use crate::moves::{DynamicChangeMove, DynamicMoveSelector};
    use solverforge_scoring::director::typed::TypedScoreDirector;
    use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};
    use solverforge_solver::heuristic::r#move::Move;
    use solverforge_solver::heuristic::selector::MoveSelector;

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

    let initial_score = director.calculate_score();
    eprintln!("Initial score: {:?}", initial_score);

    // Generate moves like MoveSelector does
    let selector = DynamicMoveSelector::new();
    let moves: Vec<DynamicChangeMove> = selector.iter_moves(&director).collect();
    eprintln!("Generated {} moves", moves.len());

    // Evaluate each move like LocalSearchPhase does
    let mut improving_moves = Vec::new();
    let mut accepted_moves = Vec::new();
    for m in &moves {
        let doable = m.is_doable(&director);
        if !doable {
            eprintln!("  NOT DOABLE: {:?}", m);
            continue;
        }

        // Use RecordingScoreDirector for automatic undo
        let move_score = {
            let mut recording = RecordingScoreDirector::new(&mut director);

            // Execute move
            m.do_move(&mut recording);

            // Calculate resulting score
            let score = recording.calculate_score();

            // Undo the move
            recording.undo_changes();

            score
        };

        // Check if accepted (Late Acceptance accepts improving OR >= late_score)
        let accepted = move_score >= initial_score; // Simplified - late acceptance would be more lenient
        if accepted {
            eprintln!(
                "  ACCEPTED: {:?} -> {:?} (improving: {})",
                m,
                move_score,
                move_score > initial_score
            );
            accepted_moves.push((m.clone(), move_score));
        }

        // Check if score improved
        if move_score > initial_score {
            improving_moves.push((m.clone(), move_score));
        }
    }

    eprintln!(
        "Accepted {} moves, {} improving",
        accepted_moves.len(),
        improving_moves.len()
    );

    // Verify score is restored after all evaluations
    let restored_score = director.calculate_score();
    eprintln!("Score after all evaluations: {:?}", restored_score);
    assert_eq!(
        restored_score, initial_score,
        "Score should be restored after move evaluations"
    );

    // We should find at least one improving move (Queen 2 row 0 -> 3)
    assert!(
        !improving_moves.is_empty(),
        "Should find at least one improving move from score {:?}",
        initial_score
    );
    eprintln!("Found {} improving moves", improving_moves.len());
}
