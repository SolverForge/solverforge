//! Shared test fixtures for SolverForge crates.
//!
//! This crate provides data types and pure functions for testing.
//! It does NOT depend on `solverforge-scoring` to avoid circular dependencies.
//!
//! - [`nqueens`] - N-Queens problem data types and conflict calculation
//! - [`task`] - Task scheduling data types
//! - [`shadow`] - Shadow variable solution data type
//! - [`entity`] - Generic entity/solution fixtures
//!
//! # Usage
//!
//! Add as a dev-dependency in your crate's `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! solverforge-test = { workspace = true }
//! ```
//!
//! Then import the fixtures you need:
//!
//! ```ignore
//! use solverforge_test::nqueens::{NQueensSolution, calculate_conflicts, create_nqueens_descriptor};
//! use solverforge_test::shadow::ShadowSolution;
//! ```

pub mod entity;
pub mod nqueens;
pub mod shadow;
pub mod task;

// Re-export commonly used types at crate root for convenience
pub use entity::{TestEntity, TestSolution};
pub use nqueens::{NQueensSolution, Queen};
pub use shadow::ShadowSolution;
pub use task::{Task, TaskSolution};
