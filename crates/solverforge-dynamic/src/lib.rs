//! Runtime-defined solution types for SolverForge.
//!
//! This crate provides dynamic solution types that implement real SolverForge traits
//! but with schemas defined at runtime rather than compile time. This enables
//! language bindings (e.g., Python) to define problems without Rust compilation.

mod constraint;
mod constraint_set;
mod descriptor;
mod eval;
mod expr;
mod manager;
mod moves;
mod solution;
mod solve;

pub use constraint::StreamOp;
pub use constraint_set::DynamicConstraintSet;
pub use descriptor::{
    DynamicDescriptor, EntityClassDef, FactClassDef, FieldDef, FieldType, ValueRangeDef,
};
pub use eval::{eval_expr, EntityRef, EvalContext};
pub use expr::Expr;
pub use manager::{DynamicSolverManager, SolveStatus};
pub use moves::{DynamicChangeMove, DynamicEntityPlacer, DynamicMoveSelector};
pub use solution::{DynamicEntity, DynamicFact, DynamicSolution, DynamicValue};
pub use solve::{solve, solve_with_controls, SolveConfig, SolveResult};
