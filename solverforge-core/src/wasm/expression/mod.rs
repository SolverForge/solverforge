mod builder;
mod substitution;
#[cfg(test)]
mod tests;

pub use builder::{Expr, FieldAccessExt};

use serde::{Deserialize, Serialize};

/// Rich expression tree for constraint predicates
///
/// This enum represents a complete expression language for building constraint predicates.
/// Expressions are serializable (via serde) for use across FFI boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum Expression {
    // ===== Literals =====
    /// Integer literal (i64) - compiles to i32 in WASM
    IntLiteral { value: i64 },

    /// 64-bit integer literal - compiles directly to i64 in WASM
    Int64Literal { value: i64 },

    /// Float literal (f64)
    FloatLiteral { value: f64 },

    /// String literal
    StringLiteral { value: String },

    /// Boolean literal
    BoolLiteral { value: bool },

    /// Null value
    Null,

    // ===== Parameter Access =====
    /// Access a function parameter by index
    Param { index: u32 },

    // ===== Field Access =====
    /// Access a field on an object
    FieldAccess {
        object: Box<Expression>,
        class_name: String,
        field_name: String,
    },

    // ===== Comparisons =====
    /// Equal comparison (==)
    Eq {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Not equal comparison (!=)
    Ne {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than comparison (<)
    Lt {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than or equal comparison (<=)
    Le {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than comparison (>)
    Gt {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than or equal comparison (>=)
    Ge {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== i64 Comparisons =====
    /// Equal comparison for i64
    Eq64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Not equal comparison for i64
    Ne64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than comparison for i64
    Lt64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Less than or equal comparison for i64
    Le64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than comparison for i64
    Gt64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Greater than or equal comparison for i64
    Ge64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== Logical Operations =====
    /// Logical AND (&&)
    And {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Logical OR (||)
    Or {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Logical NOT (!)
    Not { operand: Box<Expression> },

    /// Null check (is null)
    IsNull { operand: Box<Expression> },

    /// Not-null check (is not null)
    IsNotNull { operand: Box<Expression> },

    // ===== Arithmetic Operations =====
    /// Addition (+)
    Add {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Subtraction (-)
    Sub {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Multiplication (*)
    Mul {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Division (/)
    Div {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== i64 Arithmetic Operations =====
    /// Addition for i64
    Add64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Subtraction for i64
    Sub64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Multiplication for i64
    Mul64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Division for i64
    Div64 {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== Float Arithmetic Operations =====
    /// Float addition (f64)
    FloatAdd {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Float subtraction (f64)
    FloatSub {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Float multiplication (f64)
    FloatMul {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    /// Float division (f64)
    FloatDiv {
        left: Box<Expression>,
        right: Box<Expression>,
    },

    // ===== Math Functions =====
    /// Square root (WASM f64.sqrt intrinsic)
    Sqrt { operand: Box<Expression> },

    /// Absolute value for floats (WASM f64.abs intrinsic)
    FloatAbs { operand: Box<Expression> },

    /// Round to nearest integer (WASM f64.nearest intrinsic)
    Round { operand: Box<Expression> },

    /// Floor (WASM f64.floor intrinsic)
    Floor { operand: Box<Expression> },

    /// Ceiling (WASM f64.ceil intrinsic)
    Ceil { operand: Box<Expression> },

    /// Sine (host call)
    Sin { operand: Box<Expression> },

    /// Cosine (host call)
    Cos { operand: Box<Expression> },

    /// Arc sine (host call)
    Asin { operand: Box<Expression> },

    /// Arc cosine (host call)
    Acos { operand: Box<Expression> },

    /// Arc tangent (host call)
    Atan { operand: Box<Expression> },

    /// Arc tangent of y/x (host call)
    Atan2 {
        y: Box<Expression>,
        x: Box<Expression>,
    },

    /// Convert degrees to radians
    Radians { operand: Box<Expression> },

    /// Convert int to float
    IntToFloat { operand: Box<Expression> },

    /// Convert float to int (truncating)
    FloatToInt { operand: Box<Expression> },

    // ===== List Operations =====
    /// Check if a list contains an element
    ListContains {
        list: Box<Expression>,
        element: Box<Expression>,
    },

    /// Get the length of a collection
    Length { collection: Box<Expression> },

    /// Sum of field values over a collection
    Sum {
        collection: Box<Expression>,
        item_var_name: String,
        item_param_index: u32,
        item_class_name: String,
        accumulator_expr: Box<Expression>,
    },

    /// Access the last element of a collection
    LastElement {
        collection: Box<Expression>,
        item_class_name: String,
    },

    // ===== Host Function Calls =====
    /// Call a host-provided function
    HostCall {
        function_name: String,
        args: Vec<Expression>,
    },

    // ===== Conditional =====
    /// If-then-else conditional expression (produces i32)
    IfThenElse {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },

    /// If-then-else conditional expression (produces i64)
    IfThenElse64 {
        condition: Box<Expression>,
        then_branch: Box<Expression>,
        else_branch: Box<Expression>,
    },

    // ===== Type Conversions =====
    /// Wrap i64 to i32 (truncate)
    I64ToI32 { operand: Box<Expression> },

    /// Extend i32 to i64 (signed)
    I32ToI64 { operand: Box<Expression> },
}
