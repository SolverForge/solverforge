//! High-level solver management with zero-erasure API.
//!
//! # Zero-Erasure Design
//!
//! All types flow through generics - no Box, Arc, or dyn anywhere.
//! Runtime configuration from TOML/YAML is handled by the macro layer
//! which generates concrete types at compile time.

mod builder;
mod config;
mod solution_manager;
mod solver_factory;
mod solver_manager;

#[cfg(test)]
mod mod_tests;

pub use builder::{SolverBuildError, SolverFactoryBuilder};
pub use config::{ConstructionType, LocalSearchType, PhaseConfig};
pub use solution_manager::{Analyzable, ConstraintAnalysis, ScoreAnalysis, SolutionManager};
pub use solver_factory::{solver_factory_builder, SolverFactory};
pub use solver_manager::{Solvable, SolverManager, SolverStatus};
