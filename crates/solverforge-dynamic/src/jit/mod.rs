//! JIT compilation of Expr trees to native machine code via Cranelift.

#[cfg(test)]
mod tests;

pub mod compiler;

pub use compiler::{compile_1, compile_2, JitError, JitFn};
