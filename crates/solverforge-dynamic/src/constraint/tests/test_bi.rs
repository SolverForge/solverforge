//! Tests for bi (two-entity) constraints.

use super::*;

#[test]
fn test_row_conflict_constraint() {
    // Two queens on the same row
    let solution = make_nqueens_solution(&[0, 0, 1, 2]);

    // Build constraint using StreamOp pipeline
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 }, // Queen class
        StreamOp::Join {
            class_idx: 0, // Join with Queen (self-join)
            conditions: vec![Expr::eq(Expr::field(0, 1), Expr::field(1, 1))], // row == row
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)), // column < column
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize to compute matches and score
    let score = constraint.initialize(&solution);
    // Queens at columns 0 and 1 both have row=0, so 1 conflict
    assert_eq!(score, HardSoftScore::of_hard(-1));
}

#[test]
fn test_no_conflicts() {
    // No queens on the same row
    let solution = make_nqueens_solution(&[0, 1, 2, 3]);

    // Build constraint using StreamOp pipeline
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

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize to compute matches and score
    let score = constraint.initialize(&solution);
    assert_eq!(score, HardSoftScore::ZERO);
}

#[test]
fn test_constraint_set() {
    let solution = make_nqueens_solution(&[0, 0, 2, 2]);

    // Build constraint using StreamOp pipeline
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
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    let mut constraint_set = DynamicConstraintSet::new();
    constraint_set.add(constraint);

    // Initialize to compute matches and score
    let score = constraint_set.initialize_all(&solution);
    // Two pairs: (0,1) both row=0, (2,3) both row=2
    assert_eq!(score, HardSoftScore::of_hard(-2));
}

#[test]
fn test_bi_self_join_incremental() {
    // Test incremental scoring with on_insert
    let mut solution = make_nqueens_solution(&[0, 1, 2]);

    // Build row conflict constraint
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

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor.clone(),
    );

    // Initialize: no conflicts (rows 0, 1, 2)
    let init_score = constraint.initialize(&solution);
    assert_eq!(init_score, HardSoftScore::ZERO);

    // Insert a new queen at column 3, row 1 (conflicts with queen at column 1)
    solution.add_entity(
        0,
        DynamicEntity::new(3, vec![DynamicValue::I64(3), DynamicValue::I64(1)]),
    );
    let delta = constraint.on_insert(&solution, 3, 0);
    // New queen (col 3, row 1) conflicts with existing queen (col 1, row 1)
    assert_eq!(delta, HardSoftScore::of_hard(-1));

    // Full evaluation should match incremental result
    let full_score = constraint.evaluate(&solution);
    assert_eq!(full_score, HardSoftScore::of_hard(-1));

    // Insert another queen at column 4, row 1 (conflicts with col 1 and col 3)
    solution.add_entity(
        0,
        DynamicEntity::new(4, vec![DynamicValue::I64(4), DynamicValue::I64(1)]),
    );
    let delta2 = constraint.on_insert(&solution, 4, 0);
    // New queen (col 4, row 1) conflicts with 2 existing queens (col 1, col 3)
    assert_eq!(delta2, HardSoftScore::of_hard(-2));

    // Full evaluation: 3 total conflicts (1-3, 1-4, 3-4)
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-3));

    // Insert queen at column 5, row 0 (conflicts with col 0)
    solution.add_entity(
        0,
        DynamicEntity::new(5, vec![DynamicValue::I64(5), DynamicValue::I64(0)]),
    );
    let delta3 = constraint.on_insert(&solution, 5, 0);
    // New queen (col 5, row 0) conflicts with 1 existing queen (col 0, row 0)
    assert_eq!(delta3, HardSoftScore::of_hard(-1));

    // Full evaluation: 4 total conflicts (0-5, 1-3, 1-4, 3-4)
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-4));
}
