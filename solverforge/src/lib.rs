//! SolverForge - Constraint solver library powered by Timefold
//!
//! SolverForge is a Rust-based constraint solver library that uses WASM modules
//! and HTTP communication to solve constraint satisfaction and optimization problems.
//!
//! # Quick Start
//!
//! ```ignore
//! use solverforge::prelude::*;
//!
//! #[derive(PlanningEntity, Clone)]
//! struct Shift {
//!     #[planning_id]
//!     id: i64,
//!     #[planning_variable(value_range_provider = "employees")]
//!     employee: Option<Employee>,
//! }
//!
//! #[derive(PlanningSolution, Clone)]
//! struct Schedule {
//!     #[problem_fact_collection]
//!     #[value_range_provider(id = "employees")]
//!     employees: Vec<Employee>,
//!     #[planning_entity_collection]
//!     shifts: Vec<Shift>,
//!     #[planning_score]
//!     score: Option<HardSoftScore>,
//! }
//! ```
//!
//! The embedded solver service is automatically started when needed, eliminating
//! the need to manually manage the Java process.

pub use solverforge_core::*;
pub use solverforge_derive::*;
pub use solverforge_service::*;

/// Commonly used types for constraint solving
pub mod prelude {
    pub use solverforge_core::{
        HardSoftScore, PlanningEntity, PlanningSolution, Score, SimpleScore, SolveRequest,
        SolveResponse, Solver, SolverBuilder, SolverFactory, SolverForgeError, SolverForgeResult,
        Value,
    };
    pub use solverforge_derive::{PlanningEntity, PlanningSolution};
}
