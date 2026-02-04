//! Tests for expression evaluation.

use super::*;
use crate::descriptor::{DynamicDescriptor, EntityClassDef, FieldDef, FieldType};
use crate::expr::Expr;
use crate::solution::{DynamicEntity, DynamicSolution, DynamicValue};

fn make_test_solution() -> DynamicSolution {
    let mut desc = DynamicDescriptor::new();
    desc.add_entity_class(EntityClassDef::new(
        "Queen",
        vec![
            FieldDef::new("column", FieldType::I64),
            FieldDef::planning_variable("row", FieldType::I64, "rows"),
        ],
    ));

    let mut solution = DynamicSolution::new(desc);
    solution.add_entity(
        0,
        DynamicEntity::new(1, vec![DynamicValue::I64(0), DynamicValue::I64(2)]),
    );
    solution.add_entity(
        0,
        DynamicEntity::new(2, vec![DynamicValue::I64(1), DynamicValue::I64(2)]),
    );
    solution
}

#[test]
fn test_field_access() {
    let solution = make_test_solution();
    let tuple = [EntityRef::new(0, 0)];
    let ctx = EvalContext::new(&solution, &tuple);

    let expr = Expr::field(0, 0); // column
    let result = eval_expr(&expr, &ctx);
    assert_eq!(result.as_i64(), Some(0));

    let expr = Expr::field(0, 1); // row
    let result = eval_expr(&expr, &ctx);
    assert_eq!(result.as_i64(), Some(2));
}

#[test]
fn test_comparison() {
    let solution = make_test_solution();
    let tuple = [EntityRef::new(0, 0), EntityRef::new(0, 1)];
    let ctx = EvalContext::new(&solution, &tuple);

    // row of entity 0 == row of entity 1 (both are 2)
    let expr = Expr::eq(Expr::field(0, 1), Expr::field(1, 1));
    let result = eval_expr(&expr, &ctx);
    assert_eq!(result.as_bool(), Some(true));

    // column of entity 0 < column of entity 1
    let expr = Expr::lt(Expr::field(0, 0), Expr::field(1, 0));
    let result = eval_expr(&expr, &ctx);
    assert_eq!(result.as_bool(), Some(true));
}

#[test]
fn test_arithmetic() {
    let solution = make_test_solution();
    let tuple = [EntityRef::new(0, 0), EntityRef::new(0, 1)];
    let ctx = EvalContext::new(&solution, &tuple);

    // abs(column0 - column1) = abs(0 - 1) = 1
    let expr = Expr::abs(Expr::sub(Expr::field(0, 0), Expr::field(1, 0)));
    let result = eval_expr(&expr, &ctx);
    assert_eq!(result.as_i64(), Some(1));
}

#[test]
fn test_logical() {
    let solution = make_test_solution();
    let tuple = [EntityRef::new(0, 0), EntityRef::new(0, 1)];
    let ctx = EvalContext::new(&solution, &tuple);

    // (row0 == row1) && (column0 < column1)
    let expr = Expr::and(
        Expr::eq(Expr::field(0, 1), Expr::field(1, 1)),
        Expr::lt(Expr::field(0, 0), Expr::field(1, 0)),
    );
    let result = eval_expr(&expr, &ctx);
    assert_eq!(result.as_bool(), Some(true));
}
