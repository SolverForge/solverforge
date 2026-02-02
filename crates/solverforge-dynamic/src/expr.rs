//! Expression trees for runtime constraint evaluation.

use crate::solution::DynamicValue;

/// An expression tree node for constraint evaluation.
///
/// Expressions are evaluated against a tuple of entities (e.g., for join conditions)
/// and produce a DynamicValue result.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal value.
    Literal(DynamicValue),

    /// Parameter reference: which entity in the tuple (0 = A, 1 = B for bi-stream).
    Param(usize),

    /// Field access on the current tuple element: param_idx.field[field_idx].
    Field { param_idx: usize, field_idx: usize },

    /// Equality comparison.
    Eq(Box<Expr>, Box<Expr>),

    /// Inequality comparison.
    Ne(Box<Expr>, Box<Expr>),

    /// Less than comparison.
    Lt(Box<Expr>, Box<Expr>),

    /// Less than or equal comparison.
    Le(Box<Expr>, Box<Expr>),

    /// Greater than comparison.
    Gt(Box<Expr>, Box<Expr>),

    /// Greater than or equal comparison.
    Ge(Box<Expr>, Box<Expr>),

    /// Logical AND.
    And(Box<Expr>, Box<Expr>),

    /// Logical OR.
    Or(Box<Expr>, Box<Expr>),

    /// Logical NOT.
    Not(Box<Expr>),

    /// Absolute value.
    Abs(Box<Expr>),

    /// Addition.
    Add(Box<Expr>, Box<Expr>),

    /// Subtraction.
    Sub(Box<Expr>, Box<Expr>),

    /// Multiplication.
    Mul(Box<Expr>, Box<Expr>),

    /// Division.
    Div(Box<Expr>, Box<Expr>),

    /// Modulo.
    Mod(Box<Expr>, Box<Expr>),

    /// Negation.
    Neg(Box<Expr>),

    /// List contains check.
    Contains(Box<Expr>, Box<Expr>),

    /// Conditional expression: if cond then then_expr else else_expr.
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

    /// Check if a value is not None.
    IsNotNone(Box<Expr>),

    /// Check if a value is None.
    IsNone(Box<Expr>),

    /// Check if set/list contains a value.
    SetContains {
        set_expr: Box<Expr>,
        value_expr: Box<Expr>,
    },

    /// Extract date from datetime (days since epoch).
    DateOf(Box<Expr>),

    /// Check if two datetime ranges overlap.
    Overlaps {
        start1: Box<Expr>,
        end1: Box<Expr>,
        start2: Box<Expr>,
        end2: Box<Expr>,
    },

    /// Calculate overlap duration in minutes between two datetime ranges.
    OverlapMinutes {
        start1: Box<Expr>,
        end1: Box<Expr>,
        start2: Box<Expr>,
        end2: Box<Expr>,
    },

    /// Check if a datetime range overlaps with a date.
    OverlapsDate {
        start: Box<Expr>,
        end: Box<Expr>,
        date: Box<Expr>,
    },

    /// Calculate overlap minutes between a datetime range and a date.
    OverlapDateMinutes {
        start: Box<Expr>,
        end: Box<Expr>,
        date: Box<Expr>,
    },

    /// Minimum of two values.
    Min(Box<Expr>, Box<Expr>),

    /// Maximum of two values.
    Max(Box<Expr>, Box<Expr>),

    /// Reference to a flattened value (set by FlattenLast operation).
    FlattenedValue,
}

impl Expr {
    // Constructors for common expressions

    /// Creates a literal expression.
    pub fn literal(value: DynamicValue) -> Self {
        Expr::Literal(value)
    }

    /// Creates an integer literal.
    pub fn int(value: i64) -> Self {
        Expr::Literal(DynamicValue::I64(value))
    }

    /// Creates a boolean literal.
    pub fn bool(value: bool) -> Self {
        Expr::Literal(DynamicValue::Bool(value))
    }

    /// Creates a parameter reference.
    pub fn param(idx: usize) -> Self {
        Expr::Param(idx)
    }

    /// Creates a field access expression.
    pub fn field(param_idx: usize, field_idx: usize) -> Self {
        Expr::Field {
            param_idx,
            field_idx,
        }
    }

    /// Creates an equality expression.
    pub fn eq(left: Expr, right: Expr) -> Self {
        Expr::Eq(Box::new(left), Box::new(right))
    }

    /// Creates an inequality expression.
    pub fn ne(left: Expr, right: Expr) -> Self {
        Expr::Ne(Box::new(left), Box::new(right))
    }

    /// Creates a less-than expression.
    pub fn lt(left: Expr, right: Expr) -> Self {
        Expr::Lt(Box::new(left), Box::new(right))
    }

    /// Creates a less-than-or-equal expression.
    pub fn le(left: Expr, right: Expr) -> Self {
        Expr::Le(Box::new(left), Box::new(right))
    }

    /// Creates a greater-than expression.
    pub fn gt(left: Expr, right: Expr) -> Self {
        Expr::Gt(Box::new(left), Box::new(right))
    }

    /// Creates a greater-than-or-equal expression.
    pub fn ge(left: Expr, right: Expr) -> Self {
        Expr::Ge(Box::new(left), Box::new(right))
    }

    /// Creates a logical AND expression.
    pub fn and(left: Expr, right: Expr) -> Self {
        Expr::And(Box::new(left), Box::new(right))
    }

    /// Creates a logical OR expression.
    pub fn or(left: Expr, right: Expr) -> Self {
        Expr::Or(Box::new(left), Box::new(right))
    }

    /// Creates a logical NOT expression.
    pub fn not(expr: Expr) -> Self {
        Expr::Not(Box::new(expr))
    }

    /// Creates an absolute value expression.
    pub fn abs(expr: Expr) -> Self {
        Expr::Abs(Box::new(expr))
    }

    /// Creates an addition expression.
    pub fn add(left: Expr, right: Expr) -> Self {
        Expr::Add(Box::new(left), Box::new(right))
    }

    /// Creates a subtraction expression.
    pub fn sub(left: Expr, right: Expr) -> Self {
        Expr::Sub(Box::new(left), Box::new(right))
    }

    /// Creates a multiplication expression.
    pub fn mul(left: Expr, right: Expr) -> Self {
        Expr::Mul(Box::new(left), Box::new(right))
    }

    /// Creates a division expression.
    pub fn div(left: Expr, right: Expr) -> Self {
        Expr::Div(Box::new(left), Box::new(right))
    }

    /// Creates a modulo expression.
    pub fn modulo(left: Expr, right: Expr) -> Self {
        Expr::Mod(Box::new(left), Box::new(right))
    }

    /// Creates a negation expression.
    pub fn neg(expr: Expr) -> Self {
        Expr::Neg(Box::new(expr))
    }

    /// Creates a contains expression.
    pub fn contains(list: Expr, elem: Expr) -> Self {
        Expr::Contains(Box::new(list), Box::new(elem))
    }

    /// Creates a conditional expression.
    pub fn if_then_else(cond: Expr, then_expr: Expr, else_expr: Expr) -> Self {
        Expr::If {
            cond: Box::new(cond),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        }
    }

    /// Creates a reference field access expression.
    pub fn ref_field(ref_expr: Expr, field_idx: usize) -> Self {
        Expr::RefField {
            ref_expr: Box::new(ref_expr),
            field_idx,
        }
    }

    /// Creates an is-not-none check.
    pub fn is_not_none(expr: Expr) -> Self {
        Expr::IsNotNone(Box::new(expr))
    }

    /// Creates an is-none check.
    pub fn is_none(expr: Expr) -> Self {
        Expr::IsNone(Box::new(expr))
    }

    /// Creates a set contains check.
    pub fn set_contains(set_expr: Expr, value_expr: Expr) -> Self {
        Expr::SetContains {
            set_expr: Box::new(set_expr),
            value_expr: Box::new(value_expr),
        }
    }

    /// Creates a date-of expression (extract date from datetime).
    pub fn date_of(expr: Expr) -> Self {
        Expr::DateOf(Box::new(expr))
    }

    /// Creates an overlaps check for two datetime ranges.
    pub fn overlaps(start1: Expr, end1: Expr, start2: Expr, end2: Expr) -> Self {
        Expr::Overlaps {
            start1: Box::new(start1),
            end1: Box::new(end1),
            start2: Box::new(start2),
            end2: Box::new(end2),
        }
    }

    /// Creates an overlap minutes calculation.
    pub fn overlap_minutes(start1: Expr, end1: Expr, start2: Expr, end2: Expr) -> Self {
        Expr::OverlapMinutes {
            start1: Box::new(start1),
            end1: Box::new(end1),
            start2: Box::new(start2),
            end2: Box::new(end2),
        }
    }

    /// Creates an overlaps-date check.
    pub fn overlaps_date(start: Expr, end: Expr, date: Expr) -> Self {
        Expr::OverlapsDate {
            start: Box::new(start),
            end: Box::new(end),
            date: Box::new(date),
        }
    }

    /// Creates an overlap-date-minutes calculation.
    pub fn overlap_date_minutes(start: Expr, end: Expr, date: Expr) -> Self {
        Expr::OverlapDateMinutes {
            start: Box::new(start),
            end: Box::new(end),
            date: Box::new(date),
        }
    }

    /// Creates a minimum expression.
    pub fn min(left: Expr, right: Expr) -> Self {
        Expr::Min(Box::new(left), Box::new(right))
    }

    /// Creates a maximum expression.
    pub fn max(left: Expr, right: Expr) -> Self {
        Expr::Max(Box::new(left), Box::new(right))
    }

    /// Creates a reference to a flattened value.
    pub fn flattened_value() -> Self {
        Expr::FlattenedValue
    }
}
