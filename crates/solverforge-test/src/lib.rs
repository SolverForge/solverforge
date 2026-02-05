//! Shared test utilities for SolverForge crates.
//!
//! This crate provides common test infrastructure used across the SolverForge workspace:
//!
//! - [`nqueens`] - N-Queens problem fixtures for constraint and solver tests
//! - [`task`] - Task scheduling fixtures for selector tests
//! - [`minimal`] - Minimal score-only solutions for termination tests
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
//! use solverforge_test::nqueens::{NQueensSolution, create_nqueens_director};
//! use solverforge_test::minimal::{TestSolution, create_minimal_director};
//! ```

pub mod entity;
pub mod minimal;
pub mod nqueens;
pub mod task;

// Re-export commonly used types at crate root for convenience
pub use entity::{TestEntity, TestSolution as EntityTestSolution};
pub use minimal::{DummySolution, MinimalSolution, TestSolution};
pub use nqueens::{NQueensSolution, Queen};
pub use task::{Task, TaskSolution};
