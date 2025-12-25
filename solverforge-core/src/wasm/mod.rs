mod expression;
mod generator;
mod host_functions;
mod memory;

pub use expression::{Expr, Expression, FieldAccessExt};
pub use generator::{PredicateDefinition, WasmModuleBuilder};
pub use host_functions::{HostFunctionDef, HostFunctionRegistry, WasmType};
pub use memory::{FieldLayout, LayoutCalculator, MemoryLayout, WasmMemoryType};

// Re-export wasm_encoder::ValType for use in predicate parameter type specifications
pub use wasm_encoder::ValType;
