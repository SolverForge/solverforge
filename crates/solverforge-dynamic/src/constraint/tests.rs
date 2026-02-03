//! Tests for dynamic constraints.

use super::*;
use crate::constraint_set::DynamicConstraintSet;
use crate::descriptor::{
    DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef,
};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicValue};
use solverforge_core::score::HardSoftScore;
use solverforge_core::{ConstraintRef, ImpactType};
use solverforge_scoring::api::constraint_set::IncrementalConstraint;
use solverforge_scoring::ConstraintSet;

fn make_nqueens_solution(rows: &[i64]) -> DynamicSolution {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, rows.len() as i64));

    let mut solution = DynamicSolution::new(desc);
    for (col, &row) in rows.iter().enumerate() {
        solution.add_entity(
            0,
            DynamicEntity::new(
                col as i64,
                vec![DynamicValue::I64(col as i64), DynamicValue::I64(row)],
            ),
        );
    }
    solution
}

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
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
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
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
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
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
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
        ConstraintRef::new("row_conflict"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize: no conflicts (rows 0, 1, 2)
    let init_score = constraint.initialize(&solution);
    assert_eq!(init_score, HardSoftScore::ZERO);

    // Insert a new queen at column 3, row 1 (conflicts with queen at column 1)
    solution.add_entity(
        0,
        DynamicEntity::new(
            3,
            vec![DynamicValue::I64(3), DynamicValue::I64(1)],
        ),
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
        DynamicEntity::new(
            4,
            vec![DynamicValue::I64(4), DynamicValue::I64(1)],
        ),
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
        DynamicEntity::new(
            5,
            vec![DynamicValue::I64(5), DynamicValue::I64(0)],
        ),
    );
    let delta3 = constraint.on_insert(&solution, 5, 0);
    // New queen (col 5, row 0) conflicts with 1 existing queen (col 0, row 0)
    assert_eq!(delta3, HardSoftScore::of_hard(-1));

    // Full evaluation: 4 total conflicts (0-5, 1-3, 1-4, 3-4)
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-4));
}

#[test]
fn test_tri_self_join_incremental() {
    // Test incremental scoring for tri self-join constraint
    // Scenario: penalize triplets (a, b, c) of numbers where a + b = c
    // This tests the IncrementalTriConstraint wrapper with 3-entity tuples

    let mut descriptor = DynamicDescriptor::new();
    let number_class = descriptor.add_entity_class("Number".to_string(), vec!["value".to_string()]);

    // Initial solution: [1, 2, 3, 4]
    // Initial triplets where a + b = c: (1, 2, 3) only → 1 match
    let mut solution = DynamicSolution::new(descriptor.clone());
    solution.add_entity(number_class, DynamicEntity::new(0, vec![DynamicValue::I64(1)]));
    solution.add_entity(number_class, DynamicEntity::new(1, vec![DynamicValue::I64(2)]));
    solution.add_entity(number_class, DynamicEntity::new(2, vec![DynamicValue::I64(3)]));
    solution.add_entity(number_class, DynamicEntity::new(3, vec![DynamicValue::I64(4)]));

    // Build constraint: penalize triplets where a + b = c
    // ForEach → Join → Join → Filter(a + b = c) → Penalize
    let ops = vec![
        StreamOp::ForEach {
            class_idx: number_class,
        },
        StreamOp::Join {
            class_idx: number_class,
            conditions: vec![],
        },
        StreamOp::Join {
            class_idx: number_class,
            conditions: vec![],
        },
        StreamOp::Filter {
            predicate: Expr::eq(
                Expr::add(Expr::field(0, 0), Expr::field(1, 0)),
                Expr::field(2, 0),
            ),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];

    let mut constraint = build_from_stream_ops(
        ConstraintRef::new("sum_triplet"),
        ImpactType::Penalty,
        &ops,
        solution.descriptor().clone(),
    );

    // Initialize
    let init_score = constraint.initialize(&solution);
    // Initial: (1, 2, 3) is the only triplet where a + b = c
    assert_eq!(init_score, HardSoftScore::of_hard(-1));
    let eval_score = constraint.evaluate(&solution);
    assert_eq!(eval_score, HardSoftScore::of_hard(-1));

    // Insert number 5 at index 4
    // New triplets: (1, 4, 5), (2, 3, 5)
    solution.add_entity(
        number_class,
        DynamicEntity::new(4, vec![DynamicValue::I64(5)]),
    );
    let delta1 = constraint.on_insert(&solution, 4, number_class);
    // Delta should be -2 (two new triplets formed)
    assert_eq!(delta1, HardSoftScore::of_hard(-2));

    // Full evaluation: (1, 2, 3), (1, 4, 5), (2, 3, 5) → 3 triplets
    let full_score1 = constraint.evaluate(&solution);
    assert_eq!(full_score1, HardSoftScore::of_hard(-3));

    // Insert number 6 at index 5
    // New triplets: (1, 5, 6), (2, 4, 6)
    solution.add_entity(
        number_class,
        DynamicEntity::new(5, vec![DynamicValue::I64(6)]),
    );
    let delta2 = constraint.on_insert(&solution, 5, number_class);
    // Delta should be -2 (two new triplets formed)
    assert_eq!(delta2, HardSoftScore::of_hard(-2));

    // Full evaluation: (1,2,3), (1,4,5), (2,3,5), (1,5,6), (2,4,6) → 5 triplets
    let full_score2 = constraint.evaluate(&solution);
    assert_eq!(full_score2, HardSoftScore::of_hard(-5));

    // Insert number 7 at index 6
    // New triplets: (1, 6, 7), (2, 5, 7), (3, 4, 7)
    solution.add_entity(
        number_class,
        DynamicEntity::new(6, vec![DynamicValue::I64(7)]),
    );
    let delta3 = constraint.on_insert(&solution, 6, number_class);
    // Delta should be -3 (three new triplets formed)
    assert_eq!(delta3, HardSoftScore::of_hard(-3));

    // Full evaluation: previous 5 + 3 new → 8 triplets total
    let full_score3 = constraint.evaluate(&solution);
    assert_eq!(full_score3, HardSoftScore::of_hard(-8));
}
