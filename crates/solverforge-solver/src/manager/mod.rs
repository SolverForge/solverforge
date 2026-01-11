//! High-level solver management with zero-erasure API.
//!
//! # Zero-Erasure Design
//!
//! All types flow through generics - no Box, Arc, or dyn anywhere.
//! Runtime configuration from TOML/YAML is handled by the macro layer
//! which generates concrete types at compile time.

mod builder;
mod config;
mod phase_factory;
mod solution_manager;
mod solver_manager;

#[cfg(test)]
mod builder_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod mod_tests_integration;

pub use builder::{SolverBuildError, SolverManagerBuilder};
pub use config::{ConstructionType, LocalSearchType, PhaseConfig};
pub use phase_factory::{
    ConstructionPhaseFactory, KOptPhase, KOptPhaseBuilder, ListConstructionPhase,
    ListConstructionPhaseBuilder, LocalSearchPhaseFactory,
};
pub use solution_manager::{
    Analyzable, ConstraintAnalysis, ScoreAnalysis, Solvable, SolutionManager, SolverStatus,
};
pub use solver_manager::{solver_manager_builder, SolverManager};

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;

/// Factory trait for creating phases with zero type erasure.
///
/// Returns a concrete phase type via associated type, preserving
/// full type information through the pipeline.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
pub trait PhaseFactory<S, D>: Send + Sync
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// The concrete phase type produced by this factory.
    type Phase: Phase<S, D>;

    /// Creates a new phase instance with concrete type.
    fn create(&self) -> Self::Phase;
}
