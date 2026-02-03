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
