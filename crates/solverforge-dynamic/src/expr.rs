//! Expression trees for runtime constraint evaluation.

use std::ops::{Add, Div, Mul, Neg, Not, Sub};

use crate::solution::DynamicValue;

/// An expression tree node for constraint evaluation.
///
/// Expressions are evaluated against a tuple of entities (e.g., for join conditions)
/// and produce a DynamicValue result.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(DynamicValue),
    Param(usize),
    Field {
        param_idx: usize,
        field_idx: usize,
    },
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Abs(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),

    Contains(Box<Expr>, Box<Expr>),

    If {
        cond: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
    },

    /// Access a field on a referenced entity/fact.
    /// First evaluates the ref_expr to get a Ref/FactRef, then accesses field_idx.
    RefField {
        ref_expr: Box<Expr>,
        field_idx: usize,
    },

    IsNotNone(Box<Expr>),

    IsNone(Box<Expr>),

    SetContains {
        set_expr: Box<Expr>,
        value_expr: Box<Expr>,
    },

    DateOf(Box<Expr>),

    Overlaps {
        start1: Box<Expr>,
        end1: Box<Expr>,
        start2: Box<Expr>,
        end2: Box<Expr>,
    },

    OverlapMinutes {
        start1: Box<Expr>,
        end1: Box<Expr>,
        start2: Box<Expr>,
        end2: Box<Expr>,
    },

    OverlapsDate {
        start: Box<Expr>,
        end: Box<Expr>,
        date: Box<Expr>,
    },

    OverlapDateMinutes {
        start: Box<Expr>,
        end: Box<Expr>,
        date: Box<Expr>,
    },

    Min(Box<Expr>, Box<Expr>),

    Max(Box<Expr>, Box<Expr>),

    FlattenedValue,
}

impl Expr {
    // Constructors for common expressions

    pub fn literal(value: DynamicValue) -> Self {
        Expr::Literal(value)
    }

    pub fn int(value: i64) -> Self {
        Expr::Literal(DynamicValue::I64(value))
    }

    pub fn bool(value: bool) -> Self {
        Expr::Literal(DynamicValue::Bool(value))
    }

    pub fn param(idx: usize) -> Self {
        Expr::Param(idx)
    }

    pub fn field(param_idx: usize, field_idx: usize) -> Self {
        Expr::Field {
            param_idx,
            field_idx,
        }
    }

    pub fn eq(left: Expr, right: Expr) -> Self {
        Expr::Eq(Box::new(left), Box::new(right))
    }

    pub fn ne(left: Expr, right: Expr) -> Self {
        Expr::Ne(Box::new(left), Box::new(right))
    }

    pub fn lt(left: Expr, right: Expr) -> Self {
        Expr::Lt(Box::new(left), Box::new(right))
    }

    pub fn le(left: Expr, right: Expr) -> Self {
        Expr::Le(Box::new(left), Box::new(right))
    }

    pub fn gt(left: Expr, right: Expr) -> Self {
        Expr::Gt(Box::new(left), Box::new(right))
    }

    pub fn ge(left: Expr, right: Expr) -> Self {
        Expr::Ge(Box::new(left), Box::new(right))
    }

    pub fn and(left: Expr, right: Expr) -> Self {
        Expr::And(Box::new(left), Box::new(right))
    }

    pub fn or(left: Expr, right: Expr) -> Self {
        Expr::Or(Box::new(left), Box::new(right))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn not(expr: Expr) -> Self {
        Expr::Not(Box::new(expr))
    }

    pub fn abs(expr: Expr) -> Self {
        Expr::Abs(Box::new(expr))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(left: Expr, right: Expr) -> Self {
        Expr::Add(Box::new(left), Box::new(right))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn sub(left: Expr, right: Expr) -> Self {
        Expr::Sub(Box::new(left), Box::new(right))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn mul(left: Expr, right: Expr) -> Self {
        Expr::Mul(Box::new(left), Box::new(right))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn div(left: Expr, right: Expr) -> Self {
        Expr::Div(Box::new(left), Box::new(right))
    }

    pub fn modulo(left: Expr, right: Expr) -> Self {
        Expr::Mod(Box::new(left), Box::new(right))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn neg(expr: Expr) -> Self {
        Expr::Neg(Box::new(expr))
    }

    pub fn contains(list: Expr, elem: Expr) -> Self {
        Expr::Contains(Box::new(list), Box::new(elem))
    }

    pub fn if_then_else(cond: Expr, then_expr: Expr, else_expr: Expr) -> Self {
        Expr::If {
            cond: Box::new(cond),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        }
    }

    pub fn ref_field(ref_expr: Expr, field_idx: usize) -> Self {
        Expr::RefField {
            ref_expr: Box::new(ref_expr),
            field_idx,
        }
    }

    pub fn is_not_none(expr: Expr) -> Self {
        Expr::IsNotNone(Box::new(expr))
    }

    pub fn is_none(expr: Expr) -> Self {
        Expr::IsNone(Box::new(expr))
    }

    pub fn set_contains(set_expr: Expr, value_expr: Expr) -> Self {
        Expr::SetContains {
            set_expr: Box::new(set_expr),
            value_expr: Box::new(value_expr),
        }
    }

    pub fn date_of(expr: Expr) -> Self {
        Expr::DateOf(Box::new(expr))
    }

    pub fn overlaps(start1: Expr, end1: Expr, start2: Expr, end2: Expr) -> Self {
        Expr::Overlaps {
            start1: Box::new(start1),
            end1: Box::new(end1),
            start2: Box::new(start2),
            end2: Box::new(end2),
        }
    }

    pub fn overlap_minutes(start1: Expr, end1: Expr, start2: Expr, end2: Expr) -> Self {
        Expr::OverlapMinutes {
            start1: Box::new(start1),
            end1: Box::new(end1),
            start2: Box::new(start2),
            end2: Box::new(end2),
        }
    }

    pub fn overlaps_date(start: Expr, end: Expr, date: Expr) -> Self {
        Expr::OverlapsDate {
            start: Box::new(start),
            end: Box::new(end),
            date: Box::new(date),
        }
    }

    pub fn overlap_date_minutes(start: Expr, end: Expr, date: Expr) -> Self {
        Expr::OverlapDateMinutes {
            start: Box::new(start),
            end: Box::new(end),
            date: Box::new(date),
        }
    }

    pub fn min(left: Expr, right: Expr) -> Self {
        Expr::Min(Box::new(left), Box::new(right))
    }

    pub fn max(left: Expr, right: Expr) -> Self {
        Expr::Max(Box::new(left), Box::new(right))
    }

    pub fn flattened_value() -> Self {
        Expr::FlattenedValue
    }
}

// Implement std::ops traits for operator syntax

impl Not for Expr {
    type Output = Expr;

    fn not(self) -> Self::Output {
        Expr::Not(Box::new(self))
    }
}

impl Add for Expr {
    type Output = Expr;

    fn add(self, rhs: Self) -> Self::Output {
        Expr::Add(Box::new(self), Box::new(rhs))
    }
}

impl Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Self) -> Self::Output {
        Expr::Sub(Box::new(self), Box::new(rhs))
    }
}

impl Mul for Expr {
    type Output = Expr;

    fn mul(self, rhs: Self) -> Self::Output {
        Expr::Mul(Box::new(self), Box::new(rhs))
    }
}

impl Div for Expr {
    type Output = Expr;

    fn div(self, rhs: Self) -> Self::Output {
        Expr::Div(Box::new(self), Box::new(rhs))
    }
}

impl Neg for Expr {
    type Output = Expr;

    fn neg(self) -> Self::Output {
        Expr::Neg(Box::new(self))
    }
}
