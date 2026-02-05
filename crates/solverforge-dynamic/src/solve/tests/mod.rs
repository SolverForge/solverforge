//! Tests for dynamic solve functionality.

mod test_moves;
mod test_score_director;
mod test_solve;

use super::*;
use crate::constraint::{build_from_stream_ops, StreamOp};
use crate::descriptor::{DynamicDescriptor, EntityClassDef, FieldDef, FieldType, ValueRangeDef};
use crate::expr::Expr;
use crate::solution::DynamicEntity;
use crate::DynamicValue;
use solverforge_core::{ConstraintRef, ImpactType};

// Helper to create row conflict constraint
fn make_row_conflict_constraint(
    desc: &DynamicDescriptor,
) -> Box<
    dyn solverforge_scoring::api::constraint_set::IncrementalConstraint<
            DynamicSolution,
            HardSoftScore,
        > + Send
        + Sync,
> {
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
    build_from_stream_ops(
        ConstraintRef::new("", "row_conflict"),
        ImpactType::Penalty,
        &ops,
        desc.clone(),
    )
}

// Helper to create ascending diagonal conflict constraint
fn make_asc_diagonal_constraint(
    desc: &DynamicDescriptor,
) -> Box<
    dyn solverforge_scoring::api::constraint_set::IncrementalConstraint<
            DynamicSolution,
            HardSoftScore,
        > + Send
        + Sync,
> {
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![Expr::eq(
                Expr::sub(Expr::field(0, 1), Expr::field(1, 1)),
                Expr::sub(Expr::field(0, 0), Expr::field(1, 0)),
            )],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];
    build_from_stream_ops(
        ConstraintRef::new("", "ascending_diagonal"),
        ImpactType::Penalty,
        &ops,
        desc.clone(),
    )
}

// Helper to create descending diagonal conflict constraint
fn make_desc_diagonal_constraint(
    desc: &DynamicDescriptor,
) -> Box<
    dyn solverforge_scoring::api::constraint_set::IncrementalConstraint<
            DynamicSolution,
            HardSoftScore,
        > + Send
        + Sync,
> {
    let ops = vec![
        StreamOp::ForEach { class_idx: 0 },
        StreamOp::Join {
            class_idx: 0,
            conditions: vec![Expr::eq(
                Expr::sub(Expr::field(0, 1), Expr::field(1, 1)),
                Expr::sub(Expr::field(1, 0), Expr::field(0, 0)),
            )],
        },
        StreamOp::DistinctPair {
            ordering_expr: Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
        },
        StreamOp::Penalize {
            weight: HardSoftScore::of_hard(1),
        },
    ];
    build_from_stream_ops(
        ConstraintRef::new("", "descending_diagonal"),
        ImpactType::Penalty,
        &ops,
        desc.clone(),
    )
}

fn make_nqueens_problem(n: usize) -> (DynamicSolution, DynamicConstraintSet) {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));
    desc.add_value_range("rows", ValueRangeDef::int_range(0, n as i64));

    let mut solution = DynamicSolution::new(desc.clone());
    for col in 0..n {
        solution.add_entity(
            0,
            DynamicEntity::new(
                col as i64,
                vec![DynamicValue::I64(col as i64), DynamicValue::None],
            ),
        );
    }

    let mut constraints = DynamicConstraintSet::new();
    constraints.add(make_row_conflict_constraint(&desc));
    constraints.add(make_asc_diagonal_constraint(&desc));
    constraints.add(make_desc_diagonal_constraint(&desc));

    (solution, constraints)
}
