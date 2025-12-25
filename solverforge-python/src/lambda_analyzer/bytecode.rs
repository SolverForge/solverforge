//! Bytecode analysis for Python lambdas.
//!
//! This module provides the intermediate representation and conversion for
//! analyzing Python bytecode when source code is unavailable.

use pyo3::prelude::*;
use solverforge_core::wasm::Expression;

/// Intermediate representation for bytecode analysis.
///
/// Represents values on the stack during bytecode interpretation.
#[derive(Debug, Clone)]
pub(crate) enum BytecodeValue {
    Param(u32),
    Null,
    Bool(bool),
    Int(i64),
    String(String),
    Float(f64),
    FieldAccess {
        object: Box<BytecodeValue>,
        class_name: String,
        field_name: String,
    },
    /// Temporary marker for method reference - used during method call analysis
    MethodRef {
        object: Box<BytecodeValue>,
        method_name: String,
    },
    /// Stores an inlined expression from method inlining
    InlinedExpression(Box<Expression>),
    Eq(Box<BytecodeValue>, Box<BytecodeValue>),
    Ne(Box<BytecodeValue>, Box<BytecodeValue>),
    Lt(Box<BytecodeValue>, Box<BytecodeValue>),
    Le(Box<BytecodeValue>, Box<BytecodeValue>),
    Gt(Box<BytecodeValue>, Box<BytecodeValue>),
    Ge(Box<BytecodeValue>, Box<BytecodeValue>),
    IsNull(Box<BytecodeValue>),
    IsNotNull(Box<BytecodeValue>),
    Add(Box<BytecodeValue>, Box<BytecodeValue>),
    Sub(Box<BytecodeValue>, Box<BytecodeValue>),
    Mul(Box<BytecodeValue>, Box<BytecodeValue>),
    Div(Box<BytecodeValue>, Box<BytecodeValue>),
    Not(Box<BytecodeValue>),
    And(Box<BytecodeValue>, Box<BytecodeValue>),
    Or(Box<BytecodeValue>, Box<BytecodeValue>),
    /// Temporary markers for short-circuit evaluation pattern
    PendingAnd(Box<BytecodeValue>),
    PendingOr(Box<BytecodeValue>),
}

/// Convert BytecodeValue to Expression.
///
/// Transforms the bytecode intermediate representation into the final Expression tree.
pub(crate) fn bytecode_value_to_expression(value: BytecodeValue) -> PyResult<Expression> {
    // Use macros to reduce repetition
    macro_rules! binary {
        ($variant:ident, $l:expr, $r:expr) => {
            Ok(Expression::$variant {
                left: Box::new(bytecode_value_to_expression(*$l)?),
                right: Box::new(bytecode_value_to_expression(*$r)?),
            })
        };
    }

    macro_rules! unary {
        ($variant:ident, $operand:expr) => {
            Ok(Expression::$variant {
                operand: Box::new(bytecode_value_to_expression(*$operand)?),
            })
        };
    }

    match value {
        BytecodeValue::Param(index) => Ok(Expression::Param { index }),
        BytecodeValue::Null => Ok(Expression::Null),
        BytecodeValue::Bool(v) => Ok(Expression::BoolLiteral { value: v }),
        BytecodeValue::Int(v) => Ok(Expression::IntLiteral { value: v }),
        BytecodeValue::String(s) => Ok(Expression::StringLiteral { value: s }),
        BytecodeValue::Float(f) => Ok(Expression::FloatLiteral { value: f }),
        BytecodeValue::FieldAccess {
            object,
            class_name,
            field_name,
        } => Ok(Expression::FieldAccess {
            object: Box::new(bytecode_value_to_expression(*object)?),
            class_name,
            field_name,
        }),
        BytecodeValue::Eq(l, r) => binary!(Eq, l, r),
        BytecodeValue::Ne(l, r) => binary!(Ne, l, r),
        BytecodeValue::Lt(l, r) => binary!(Lt, l, r),
        BytecodeValue::Le(l, r) => binary!(Le, l, r),
        BytecodeValue::Gt(l, r) => binary!(Gt, l, r),
        BytecodeValue::Ge(l, r) => binary!(Ge, l, r),
        BytecodeValue::IsNull(operand) => unary!(IsNull, operand),
        BytecodeValue::IsNotNull(operand) => unary!(IsNotNull, operand),
        BytecodeValue::Add(l, r) => binary!(Add, l, r),
        BytecodeValue::Sub(l, r) => binary!(Sub, l, r),
        BytecodeValue::Mul(l, r) => binary!(Mul, l, r),
        BytecodeValue::Div(l, r) => binary!(Div, l, r),
        BytecodeValue::Not(operand) => unary!(Not, operand),
        BytecodeValue::And(l, r) => binary!(And, l, r),
        BytecodeValue::Or(l, r) => binary!(Or, l, r),
        BytecodeValue::MethodRef { .. } => {
            // Method reference that wasn't inlined - shouldn't reach here
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze lambda: unresolved method reference.",
            ))
        }
        BytecodeValue::InlinedExpression(expr) => {
            // Successfully inlined expression
            Ok(*expr)
        }
        BytecodeValue::PendingAnd(_) | BytecodeValue::PendingOr(_) => {
            // These should have been resolved - incomplete boolean expression
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot analyze lambda: incomplete boolean expression.",
            ))
        }
    }
}
