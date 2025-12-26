//! Type definitions for AST analysis.
//!
//! This module defines types used throughout the lambda analyzer
//! for type inference and expression conversion.

/// Inferred type from AST analysis.
///
/// This enum represents the type of a value as determined by analyzing
/// the AST structure BEFORE converting to expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferredType {
    /// 32-bit integer (default for integer literals and operations)
    I32,
    /// 64-bit integer (datetime fields, large integers, timedelta)
    I64,
    /// 64-bit floating point
    F64,
    /// Boolean
    Bool,
    /// String
    String,
    /// Null/None
    Null,
    /// Unknown type (couldn't be determined)
    Unknown,
}

impl InferredType {
    /// Promote two numeric types to a common type for binary operations.
    /// - If either is F64, result is F64
    /// - If either is I64, result is I64
    /// - Otherwise I32
    pub fn promote(self, other: InferredType) -> InferredType {
        match (self, other) {
            // Null is transparent - takes the type of the other operand
            // This handles "field is None" where field type determines the comparison
            (InferredType::Null, t) | (t, InferredType::Null) => t,
            // Float propagates
            (InferredType::F64, _) | (_, InferredType::F64) => InferredType::F64,
            // I64 propagates
            (InferredType::I64, _) | (_, InferredType::I64) => InferredType::I64,
            // Both I32
            (InferredType::I32, InferredType::I32) => InferredType::I32,
            // Unknown cases - be conservative
            _ => InferredType::Unknown,
        }
    }
}

/// Expected type context for expression conversion.
///
/// This is derived from InferredType and passed to conversion functions
/// to ensure literals are emitted with the correct type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExpectedType {
    /// No specific expectation - use default types (I32 for integers)
    #[default]
    Any,
    /// Expect i32 (standard integers)
    I32,
    /// Expect i64 (for datetime fields, large integers)
    I64,
    /// Expect f64 (for floating point)
    F64,
}
