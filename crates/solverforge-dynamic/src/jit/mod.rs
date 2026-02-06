//! JIT compilation of Expr trees to native machine code via Cranelift.
//!
//! This module compiles constraint expressions into native function pointers,
//! eliminating the AST-walking interpreter overhead that dominates the dynamic path.
//!
//! # Entity Memory Layout
//!
//! JIT-compiled functions operate on a flat `&[i64]` entity buffer where each field
//! occupies one i64 slot. The `DynamicValue` tagged union is bypassed entirely.
//!
//! # Function Signatures
//!
//! - **Uni filter**: `fn(entity_a: *const i64) -> bool`
//! - **Uni key**: `fn(entity_a: *const i64) -> i64`
//! - **Bi filter**: `fn(entity_a: *const i64, entity_b: *const i64) -> bool`
//! - **Bi key**: `fn(entity_a: *const i64) -> i64`
//! - **Bi weight**: `fn(entity_a: *const i64, entity_b: *const i64) -> i64`

#[cfg(test)]
mod tests;

mod compiler;

pub use compiler::{JitCompiler, JitError, CompiledBiFilter, CompiledUniKey, CompiledBiWeight};
