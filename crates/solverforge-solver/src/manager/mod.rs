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
mod solver_factory;
mod solver_manager;

#[cfg(test)]
mod mod_tests;

pub use builder::{SolverBuildError, SolverFactoryBuilder};
pub use config::{ConstructionType, LocalSearchType, PhaseConfig};
pub use phase_factory::{
    ConstructionPhaseFactory, KOptPhase, KOptPhaseBuilder, ListConstructionPhase,
    ListConstructionPhaseBuilder, LocalSearchPhaseFactory,
};
pub use solution_manager::{Analyzable, ConstraintAnalysis, ScoreAnalysis, SolutionManager};
pub use solver_factory::{solver_factory_builder, SolverFactory};
pub use solver_manager::{Solvable, SolverManager, SolverStatus};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use crate::phase::Phase;

/// Factory trait for creating phases with zero type erasure.
///
/// Returns a concrete phase type via associated type, preserving
/// full type information through the pipeline.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `C` - The constraint set type
pub trait PhaseFactory<S, C>: Send + Sync
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// The concrete phase type produced by this factory.
    type Phase: Phase<S, C>;

    /// Creates a new phase instance with concrete type.
    fn create(&self) -> Self::Phase;
}
