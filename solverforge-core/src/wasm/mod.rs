mod expr_builder;
mod expression;
mod generator;
mod host_functions;
mod memory;

pub use expr_builder::{Expr, FieldAccessExt};
pub use expression::Expression;
pub use generator::{
    Comparison, FieldAccess, PredicateBody, PredicateDefinition, WasmModuleBuilder,
};
pub use host_functions::{HostFunctionDef, HostFunctionRegistry, WasmType};
pub use memory::{FieldLayout, LayoutCalculator, MemoryLayout, WasmMemoryType};

// Re-export wasm_encoder::ValType for use in predicate parameter type specifications
pub use wasm_encoder::ValType;
