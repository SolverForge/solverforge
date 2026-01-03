//! Employee Scheduling Quickstart for SolverForge
//!
//! This library provides the domain model, typed constraints, and solver for
//! employee scheduling optimization.
//!
//! Uses zero-erasure typed constraints via `TypedScoreDirector`.

pub mod api;
pub mod console;
pub mod constraints;
pub mod demo_data;
pub mod domain;
pub mod incremental_constraints;
pub mod solver;
pub mod typed_solver;
