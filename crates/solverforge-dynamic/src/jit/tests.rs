//! Tests for JIT-compiled expressions.

use super::compiler::{compile_1, compile_2, compile_n};
use crate::expr::Expr;
use crate::NONE_SENTINEL;

fn entity(fields: &[i64]) -> Vec<i64> {
    fields.to_vec()
}

#[test]
fn test_uni_key_field_access() {
    let f = compile_1(&Expr::field(0, 2));
    let e = entity(&[10, 20, 30, 40]);
    assert_eq!(f.call_1(e.as_ptr()), 30);
}

#[test]
fn test_uni_key_literal() {
    let f = compile_1(&Expr::int(42));
    assert_eq!(f.call_1(entity(&[0]).as_ptr()), 42);
}

#[test]
fn test_bi_filter_field_equality() {
    let f = compile_2(&Expr::eq(Expr::field(0, 1), Expr::field(1, 1)));
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
    ));
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
    let f = compile_2(&Expr::abs(Expr::sub(Expr::field(0, 1), Expr::field(1, 1))));
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
    let f = compile_2(&Expr::is_not_none(Expr::field(0, 1)));
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
    ));
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
    let min_f = compile_2(&Expr::min(Expr::field(0, 0), Expr::field(1, 0)));
    let max_f = compile_2(&Expr::max(Expr::field(0, 0), Expr::field(1, 0)));
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
    ));
    assert_eq!(f.call_1(entity(&[10]).as_ptr()), 100);
    assert_eq!(f.call_1(entity(&[3]).as_ptr()), 0);
}

#[test]
fn test_nqueens_row_key() {
    let f = compile_1(&Expr::field(0, 1));
    assert_eq!(f.call_1(entity(&[3, 5]).as_ptr()), 5);
}

#[test]
fn test_nqueens_ascending_diagonal_key() {
    let f = compile_1(&Expr::sub(Expr::field(0, 1), Expr::field(0, 0)));
    assert_eq!(f.call_1(entity(&[2, 5]).as_ptr()), 3);
    assert_eq!(f.call_1(entity(&[0, 3]).as_ptr()), 3);
}

// ---------------------------------------------------------------------------
// N-ary compile_n tests — generalized indirect pointer-array calling convention
// ---------------------------------------------------------------------------

#[test]
fn test_compile_n_1_matches_compile_1() {
    let f1 = compile_1(&Expr::field(0, 1));
    let fn1 = compile_n(&Expr::field(0, 1), 1);
    let e = entity(&[10, 20, 30]);
    assert_eq!(f1.call_1(e.as_ptr()), fn1.call_1(e.as_ptr()));
}

#[test]
fn test_compile_n_2_matches_compile_2() {
    let expr = Expr::eq(Expr::field(0, 0), Expr::field(1, 0));
    let f2 = compile_2(&expr);
    let fn2 = compile_n(&expr, 2);
    let a = entity(&[42, 0]);
    let b = entity(&[42, 1]);
    let c = entity(&[99, 2]);
    assert_eq!(
        f2.call_2(a.as_ptr(), b.as_ptr()),
        fn2.call_2(a.as_ptr(), b.as_ptr())
    );
    assert_eq!(
        f2.call_2(a.as_ptr(), c.as_ptr()),
        fn2.call_2(a.as_ptr(), c.as_ptr())
    );
}

#[test]
fn test_compile_n_3_tri_filter() {
    // Tri filter: A.field0 == B.field0 && B.field0 == C.field0
    let expr = Expr::and(
        Expr::eq(Expr::field(0, 0), Expr::field(1, 0)),
        Expr::eq(Expr::field(1, 0), Expr::field(2, 0)),
    );
    let f = compile_n(&expr, 3);
    let a = entity(&[5, 10]);
    let b = entity(&[5, 20]);
    let c = entity(&[5, 30]);
    let d = entity(&[6, 40]);
    let ptrs_match = [a.as_ptr(), b.as_ptr(), c.as_ptr()];
    let ptrs_no = [a.as_ptr(), b.as_ptr(), d.as_ptr()];
    assert_ne!(f.call_n(&ptrs_match), 0); // all field0 == 5
    assert_eq!(f.call_n(&ptrs_no), 0); // d.field0 == 6
}

#[test]
fn test_compile_n_4_quad_weight() {
    // Quad weight: A.f0 + B.f0 + C.f0 + D.f0
    let expr = Expr::add(
        Expr::add(Expr::field(0, 0), Expr::field(1, 0)),
        Expr::add(Expr::field(2, 0), Expr::field(3, 0)),
    );
    let f = compile_n(&expr, 4);
    let a = entity(&[10]);
    let b = entity(&[20]);
    let c = entity(&[30]);
    let d = entity(&[40]);
    let ptrs = [a.as_ptr(), b.as_ptr(), c.as_ptr(), d.as_ptr()];
    assert_eq!(f.call_n(&ptrs), 100);
}

#[test]
fn test_compile_n_5_penta() {
    // Penta: A.f0 * B.f0 - C.f0 + D.f0 - E.f0
    let expr = Expr::sub(
        Expr::add(
            Expr::sub(
                Expr::mul(Expr::field(0, 0), Expr::field(1, 0)),
                Expr::field(2, 0),
            ),
            Expr::field(3, 0),
        ),
        Expr::field(4, 0),
    );
    let f = compile_n(&expr, 5);
    let a = entity(&[3]);
    let b = entity(&[4]);
    let c = entity(&[2]);
    let d = entity(&[10]);
    let e = entity(&[5]);
    let ptrs = [a.as_ptr(), b.as_ptr(), c.as_ptr(), d.as_ptr(), e.as_ptr()];
    // 3*4 - 2 + 10 - 5 = 12 - 2 + 10 - 5 = 15
    assert_eq!(f.call_n(&ptrs), 15);
}
