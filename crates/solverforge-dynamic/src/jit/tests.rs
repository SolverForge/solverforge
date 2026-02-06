//! Tests for JIT-compiled expressions.

use super::compiler::{compile_1, compile_2};
use crate::expr::Expr;
use crate::NONE_SENTINEL;

fn entity(fields: &[i64]) -> Vec<i64> {
    fields.to_vec()
}

#[test]
fn test_uni_key_field_access() {
    let f = compile_1(&Expr::field(0, 2)).unwrap();
    let e = entity(&[10, 20, 30, 40]);
    assert_eq!(f.call_1(e.as_ptr()), 30);
}

#[test]
fn test_uni_key_literal() {
    let f = compile_1(&Expr::int(42)).unwrap();
    assert_eq!(f.call_1(entity(&[0]).as_ptr()), 42);
}

#[test]
fn test_bi_filter_field_equality() {
    let f = compile_2(&Expr::eq(Expr::field(0, 1), Expr::field(1, 1))).unwrap();
    let a = entity(&[0, 7, 99]);
    let b = entity(&[1, 7, 50]);
    let c = entity(&[2, 8, 50]);
    assert_ne!(f.call_2(a.as_ptr(), b.as_ptr()), 0); // 7 == 7
    assert_eq!(f.call_2(a.as_ptr(), c.as_ptr()), 0); // 7 != 8
}

#[test]
fn test_bi_filter_and() {
    let f = compile_2(&Expr::and(
        Expr::eq(Expr::field(0, 0), Expr::field(1, 0)),
        Expr::lt(Expr::field(0, 1), Expr::field(1, 1)),
    ))
    .unwrap();
    let a = entity(&[5, 10]);
    let b = entity(&[5, 20]);
    let c = entity(&[5, 5]);
    let d = entity(&[6, 20]);
    assert_ne!(f.call_2(a.as_ptr(), b.as_ptr()), 0); // 5==5 && 10<20
    assert_eq!(f.call_2(a.as_ptr(), c.as_ptr()), 0); // 5==5 && 10<5
    assert_eq!(f.call_2(a.as_ptr(), d.as_ptr()), 0); // 5!=6
}

#[test]
fn test_bi_weight_arithmetic() {
    let f = compile_2(&Expr::abs(Expr::sub(Expr::field(0, 1), Expr::field(1, 1)))).unwrap();
    assert_eq!(
        f.call_2(entity(&[0, 10]).as_ptr(), entity(&[1, 3]).as_ptr()),
        7
    );
    assert_eq!(
        f.call_2(entity(&[2, 3]).as_ptr(), entity(&[3, 10]).as_ptr()),
        7
    );
}

#[test]
fn test_is_none_sentinel() {
    let f = compile_2(&Expr::is_not_none(Expr::field(0, 1))).unwrap();
    let dummy = entity(&[0, 0]);
    assert_ne!(f.call_2(entity(&[0, 42]).as_ptr(), dummy.as_ptr()), 0);
    assert_eq!(
        f.call_2(entity(&[0, NONE_SENTINEL]).as_ptr(), dummy.as_ptr()),
        0
    );
}

#[test]
fn test_overlaps() {
    let f = compile_2(&Expr::overlaps(
        Expr::field(0, 0),
        Expr::field(0, 1),
        Expr::field(1, 0),
        Expr::field(1, 1),
    ))
    .unwrap();
    assert_ne!(
        f.call_2(entity(&[0, 10]).as_ptr(), entity(&[5, 15]).as_ptr()),
        0
    );
    assert_eq!(
        f.call_2(entity(&[0, 10]).as_ptr(), entity(&[10, 20]).as_ptr()),
        0
    );
    assert_eq!(
        f.call_2(entity(&[0, 10]).as_ptr(), entity(&[11, 20]).as_ptr()),
        0
    );
}

#[test]
fn test_min_max() {
    let min_f = compile_2(&Expr::min(Expr::field(0, 0), Expr::field(1, 0))).unwrap();
    let max_f = compile_2(&Expr::max(Expr::field(0, 0), Expr::field(1, 0))).unwrap();
    let a = entity(&[3]);
    let b = entity(&[7]);
    assert_eq!(min_f.call_2(a.as_ptr(), b.as_ptr()), 3);
    assert_eq!(max_f.call_2(a.as_ptr(), b.as_ptr()), 7);
}

#[test]
fn test_if_then_else() {
    let f = compile_1(&Expr::if_then_else(
        Expr::gt(Expr::field(0, 0), Expr::int(5)),
        Expr::int(100),
        Expr::int(0),
    ))
    .unwrap();
    assert_eq!(f.call_1(entity(&[10]).as_ptr()), 100);
    assert_eq!(f.call_1(entity(&[3]).as_ptr()), 0);
}

#[test]
fn test_nqueens_row_key() {
    let f = compile_1(&Expr::field(0, 1)).unwrap();
    assert_eq!(f.call_1(entity(&[3, 5]).as_ptr()), 5);
}

#[test]
fn test_nqueens_ascending_diagonal_key() {
    let f = compile_1(&Expr::sub(Expr::field(0, 1), Expr::field(0, 0))).unwrap();
    assert_eq!(f.call_1(entity(&[2, 5]).as_ptr()), 3);
    assert_eq!(f.call_1(entity(&[0, 3]).as_ptr()), 3);
}
