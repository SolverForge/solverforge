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

/// Replicates the exact employee scheduling scenario:
/// Bi self-join on employee_idx, evenly distributed, tests whether
/// the score delta is nonzero when moving between uneven groups.
#[test]
fn test_employee_scheduling_score_delta() {
    use crate::moves::DynamicChangeMove;
    use solverforge_scoring::director::typed::TypedScoreDirector;
    use solverforge_scoring::{RecordingScoreDirector, ScoreDirector};
    use solverforge_solver::heuristic::r#move::Move;

    // Create a small schedule: 10 shifts, 3 employees
    // Assign: emp0 gets 4 shifts (0,1,2,3), emp1 gets 3 shifts (4,5,6), emp2 gets 3 shifts (7,8,9)
    // Pairs: emp0=C(4,2)=6, emp1=C(3,2)=3, emp2=C(3,2)=3 → total=12 hard penalty
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Shift",
        vec![
            FieldDef::new("id", FieldType::I64),
            FieldDef::planning_variable("employee_idx", FieldType::I64, "employees"),
        ],
    ));
    desc.add_value_range("employees", ValueRangeDef::int_range(0, 3));

    let assignments = [0, 0, 0, 0, 1, 1, 1, 2, 2, 2]; // 4-3-3
    let mut solution = DynamicSolution::new(desc.clone());
    for (i, &emp) in assignments.iter().enumerate() {
        solution.add_entity(
            0,
            DynamicEntity::new(
                i as i64,
                vec![DynamicValue::I64(i as i64), DynamicValue::I64(emp)],
            ),
        );
    }

    // Constraint: penalize all pairs on same employee
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];
    let constraint = build_from_stream_ops(
        ConstraintRef::new("", "same_employee"),
        ImpactType::Penalty,
        &ops,
        desc.clone(),
    );
    let mut constraints = DynamicConstraintSet::new();
    constraints.add(constraint);

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
    eprintln!("EMPLOYEE SCHED initial_score={}", initial_score);
    // emp0: C(4,2)=6, emp1: C(3,2)=3, emp2: C(3,2)=3 → -12hard
    assert_eq!(initial_score, HardSoftScore::of_hard(-12));

    // Move shift 0 from emp0 (group of 4) to emp2 (group of 3)
    // After: emp0=3 shifts (1,2,3), emp1=3 (4,5,6), emp2=4 (7,8,9,0)
    // Pairs: emp0=C(3,2)=3, emp1=C(3,2)=3, emp2=C(4,2)=6 → still -12hard
    // Net delta should be 0 for this specific move (4→3 and 3→4 swaps pair count)
    let move_4_to_3 = DynamicChangeMove::new(0, 0, 1, "employee_idx", DynamicValue::I64(2));

    let move_score_4_to_3 = {
        let mut recording = RecordingScoreDirector::new(&mut director);
        move_4_to_3.do_move(&mut recording);
        let score = recording.calculate_score();
        recording.undo_changes();
        score
    };
    eprintln!("EMPLOYEE SCHED move 4→3 score={}", move_score_4_to_3);
    assert_eq!(
        move_score_4_to_3,
        HardSoftScore::of_hard(-12),
        "4→3 should be net zero delta"
    );

    // Move shift 0 from emp0 (group of 4) to emp1 (group of 3)
    // After: emp0=3 (1,2,3), emp1=4 (4,5,6,0), emp2=3 (7,8,9) → still -12hard
    let move_4_to_3b = DynamicChangeMove::new(0, 0, 1, "employee_idx", DynamicValue::I64(1));

    let move_score_4_to_3b = {
        let mut recording = RecordingScoreDirector::new(&mut director);
        move_4_to_3b.do_move(&mut recording);
        let score = recording.calculate_score();
        recording.undo_changes();
        score
    };
    eprintln!("EMPLOYEE SCHED move 4→3b score={}", move_score_4_to_3b);
    assert_eq!(
        move_score_4_to_3b,
        HardSoftScore::of_hard(-12),
        "4→3 should be net zero delta"
    );

    // NOW: move shift 4 from emp1 (group of 3) to emp0 (group of 4)
    // After: emp0=5 (0,1,2,3,4), emp1=2 (5,6), emp2=3 (7,8,9)
    // Pairs: emp0=C(5,2)=10, emp1=C(2,2)=1, emp2=C(3,2)=3 → -14hard (WORSE)
    let move_3_to_4 = DynamicChangeMove::new(0, 4, 1, "employee_idx", DynamicValue::I64(0));

    let move_score_3_to_4 = {
        let mut recording = RecordingScoreDirector::new(&mut director);
        move_3_to_4.do_move(&mut recording);
        let score = recording.calculate_score();
        recording.undo_changes();
        score
    };
    eprintln!("EMPLOYEE SCHED move 3→4 score={}", move_score_3_to_4);
    assert_eq!(
        move_score_3_to_4,
        HardSoftScore::of_hard(-14),
        "3→4 should worsen to -14"
    );

    // Move shift 3 from emp0 (group of 4) to NEW empty group won't work (only 3 employees)
    // Instead: demonstrate an IMPROVING move from 4-3-3 to 3-4-3
    // Actually with this constraint (all pairs on same employee), the optimum is 4-3-3 or 3-4-3 or 3-3-4
    // These all give -12. We CANNOT improve beyond -12 with 10 shifts and 3 employees.
    // C(4,2)+C(3,2)+C(3,2)=6+3+3=12 is already the minimum for 10 items in 3 bins.
    //
    // So the LOCAL SEARCH genuinely cannot improve the score.
    // This confirms the score-not-improving observation IS correct behavior for this constraint.
    eprintln!("CONFIRMED: With a 'penalize all pairs on same employee' constraint,");
    eprintln!("the construction heuristic finds the optimal distribution.");
    eprintln!("Local search cannot improve because C(k,2) is minimized when groups are balanced.");
    eprintln!("The fix is in the Python constraint definition, not the solver wiring.");
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
