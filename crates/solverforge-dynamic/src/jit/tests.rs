//! Tests for JIT-compiled expressions.

use super::compiler::{JitCompiler, NONE_SENTINEL};
use crate::expr::Expr;

/// Helper: build a flat i64 entity buffer from a slice.
fn entity(fields: &[i64]) -> Vec<i64> {
    fields.to_vec()
}

#[test]
fn test_uni_key_field_access() {
    // Key = field 2 of entity
    let expr = Expr::field(0, 2);
    let key = JitCompiler::compile_uni_key(&expr).unwrap();

    let e = entity(&[10, 20, 30, 40]);
    assert_eq!(key.call(e.as_ptr()), 30);
}

#[test]
fn test_uni_key_literal() {
    let expr = Expr::int(42);
    let key = JitCompiler::compile_uni_key(&expr).unwrap();
    let e = entity(&[0]);
    assert_eq!(key.call(e.as_ptr()), 42);
}

#[test]
fn test_bi_filter_field_equality() {
    // A.field[1] == B.field[1]
    let expr = Expr::eq(Expr::field(0, 1), Expr::field(1, 1));
    let filter = JitCompiler::compile_bi_filter(&expr).unwrap();

    let a = entity(&[0, 7, 99]);
    let b = entity(&[1, 7, 50]);
    let c = entity(&[2, 8, 50]);

    assert!(filter.call(a.as_ptr(), b.as_ptr())); // 7 == 7
    assert!(!filter.call(a.as_ptr(), c.as_ptr())); // 7 != 8
}

#[test]
fn test_bi_filter_and() {
    // A.field[0] == B.field[0] AND A.field[1] < B.field[1]
    let expr = Expr::and(
        Expr::eq(Expr::field(0, 0), Expr::field(1, 0)),
        Expr::lt(Expr::field(0, 1), Expr::field(1, 1)),
    );
    let filter = JitCompiler::compile_bi_filter(&expr).unwrap();

    let a = entity(&[5, 10]);
    let b = entity(&[5, 20]);
    let c = entity(&[5, 5]);
    let d = entity(&[6, 20]);

    assert!(filter.call(a.as_ptr(), b.as_ptr())); // 5==5 && 10<20
    assert!(!filter.call(a.as_ptr(), c.as_ptr())); // 5==5 && 10<5 = false
    assert!(!filter.call(a.as_ptr(), d.as_ptr())); // 5!=6
}

#[test]
fn test_bi_weight_arithmetic() {
    // abs(A.field[1] - B.field[1])
    let expr = Expr::abs(Expr::sub(Expr::field(0, 1), Expr::field(1, 1)));
    let weight = JitCompiler::compile_bi_weight(&expr).unwrap();

    let a = entity(&[0, 10]);
    let b = entity(&[1, 3]);
    assert_eq!(weight.call(a.as_ptr(), b.as_ptr()), 7);

    let c = entity(&[2, 3]);
    let d = entity(&[3, 10]);
    assert_eq!(weight.call(c.as_ptr(), d.as_ptr()), 7);
}

#[test]
fn test_is_none_sentinel() {
    // IsNotNone(A.field[1])
    let expr = Expr::is_not_none(Expr::field(0, 1));
    let filter_expr = expr;
    let filter = JitCompiler::compile_bi_filter(&filter_expr).unwrap();

    let assigned = entity(&[0, 42]);
    let unassigned = entity(&[0, NONE_SENTINEL]);
    let dummy = entity(&[0, 0]);

    // BiFilter takes two params; we only care about A (param 0)
    assert!(filter.call(assigned.as_ptr(), dummy.as_ptr()));
    assert!(!filter.call(unassigned.as_ptr(), dummy.as_ptr()));
}

#[test]
fn test_overlaps() {
    // Overlaps(A.field[0], A.field[1], B.field[0], B.field[1])
    let expr = Expr::overlaps(
        Expr::field(0, 0),
        Expr::field(0, 1),
        Expr::field(1, 0),
        Expr::field(1, 1),
    );
    let filter = JitCompiler::compile_bi_filter(&expr).unwrap();

    let a = entity(&[0, 10]); // range [0, 10)
    let b = entity(&[5, 15]); // range [5, 15) → overlaps
    let c = entity(&[10, 20]); // range [10, 20) → no overlap (max(0,10)=10, min(10,20)=10, 10<10 false)
    let d = entity(&[11, 20]); // range [11, 20) → no overlap

    assert!(filter.call(a.as_ptr(), b.as_ptr()));
    assert!(!filter.call(a.as_ptr(), c.as_ptr()));
    assert!(!filter.call(a.as_ptr(), d.as_ptr()));
}

#[test]
fn test_min_max() {
    // min(A.field[0], B.field[0])
    let min_expr = Expr::min(Expr::field(0, 0), Expr::field(1, 0));
    let max_expr = Expr::max(Expr::field(0, 0), Expr::field(1, 0));

    let min_fn = JitCompiler::compile_bi_weight(&min_expr).unwrap();
    let max_fn = JitCompiler::compile_bi_weight(&max_expr).unwrap();

    let a = entity(&[3]);
    let b = entity(&[7]);

    assert_eq!(min_fn.call(a.as_ptr(), b.as_ptr()), 3);
    assert_eq!(max_fn.call(a.as_ptr(), b.as_ptr()), 7);
}

#[test]
fn test_if_then_else() {
    // if A.field[0] > 5 then 100 else 0
    let expr = Expr::if_then_else(
        Expr::gt(Expr::field(0, 0), Expr::int(5)),
        Expr::int(100),
        Expr::int(0),
    );
    let key = JitCompiler::compile_uni_key(&expr).unwrap();

    let high = entity(&[10]);
    let low = entity(&[3]);

    assert_eq!(key.call(high.as_ptr()), 100);
    assert_eq!(key.call(low.as_ptr()), 0);
}

#[test]
fn test_nqueens_row_key() {
    // Key for row conflict: entity.field[1] (the row)
    let expr = Expr::field(0, 1);
    let key = JitCompiler::compile_uni_key(&expr).unwrap();

    // Queen at column 3, row 5
    let queen = entity(&[3, 5]);
    assert_eq!(key.call(queen.as_ptr()), 5);
}

#[test]
fn test_nqueens_ascending_diagonal_key() {
    // Key for ascending diagonal: row - column = field[1] - field[0]
    let expr = Expr::sub(Expr::field(0, 1), Expr::field(0, 0));
    let key = JitCompiler::compile_uni_key(&expr).unwrap();

    // Queen at column 2, row 5 → diagonal = 3
    let queen = entity(&[2, 5]);
    assert_eq!(key.call(queen.as_ptr()), 3);

    // Queen at column 0, row 3 → diagonal = 3 (same diagonal)
    let queen2 = entity(&[0, 3]);
    assert_eq!(key.call(queen2.as_ptr()), 3);
}
