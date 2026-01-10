//! High-level solver management with ergonomic API.
//!
//! The `SolverManager` provides a simplified interface for configuring and
//! running solvers. It stores configuration and can create solvers on demand.
//!
//! # Overview
//!
//! The manager module provides:
//! - [`SolverManager`]: High-level solver configuration and creation
//! - [`SolverPhaseFactory`]: Trait for creating fresh phase instances per solve
//! - [`CloneablePhaseFactory`]: Factory that clones a prototype phase
//! - [`ClosurePhaseFactory`]: Factory using a closure
//!
//! # Runtime Configuration
//!
//! This module uses `Box<dyn Phase<S, D>>` at the API boundary to support
//! runtime configuration from TOML/YAML files. The zero-erasure architecture
//! is preserved for static configuration via the macro-generated tuple
//! implementations in `Solver`.

mod builder;
mod config;
mod phase_factory;
mod solver_manager;

#[cfg(test)]
mod builder_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod mod_tests_integration;
#[cfg(test)]
mod phase_factory_tests;
#[cfg(test)]
mod phase_factory_tests_localsearch;

pub use builder::SolverManagerBuilder;
pub use config::{ConstructionType, LocalSearchType, PhaseConfig};
pub use phase_factory::{ConstructionPhaseFactory, LocalSearchPhaseFactory};
pub use solver_manager::SolverManager;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;

/// Factory trait for creating phases at runtime.
///
/// Phase factories allow creating fresh phase instances for each solve,
/// ensuring clean state between solves. This is essential because phases
/// often maintain internal state (like step counters or tabu lists) that
/// must be reset for each new solve.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
///
/// # Implementing SolverPhaseFactory
///
/// There are two common ways to implement this trait:
///
/// 1. Use [`CloneablePhaseFactory`] for phases that implement `Clone`
/// 2. Use [`ClosurePhaseFactory`] with a closure that creates phases
pub trait SolverPhaseFactory<S, D>: Send + Sync
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// Creates a new phase instance.
    ///
    /// This method is called once per solve to create a fresh phase
    /// with clean state.
    fn create_phase(&self) -> Box<dyn Phase<S, D>>;
}

/// A simple phase factory that clones a prototype phase.
///
/// This factory stores a prototype phase and clones it for each solve.
/// Use this when your phase type implements `Clone`.
///
/// # Type Parameters
///
/// * `P` - The phase type (must implement `Clone`)
pub struct CloneablePhaseFactory<P> {
    prototype: P,
}

impl<P: Clone> CloneablePhaseFactory<P> {
    /// Creates a new factory from a prototype phase.
    ///
    /// The prototype will be cloned each time [`SolverPhaseFactory::create_phase()`]
    /// is called.
    pub fn new(prototype: P) -> Self {
        Self { prototype }
    }
}

impl<S, D, P> SolverPhaseFactory<S, D> for CloneablePhaseFactory<P>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D> + Clone + Send + Sync + 'static,
{
    fn create_phase(&self) -> Box<dyn Phase<S, D>> {
        Box::new(self.prototype.clone())
    }
}

/// A phase factory using a closure.
///
/// This factory uses a closure to create phase instances. This is useful
/// when the phase creation requires complex logic or external dependencies
/// that cannot be easily cloned.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
/// * `F` - The closure type
pub struct ClosurePhaseFactory<S, D, F>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    F: Fn() -> Box<dyn Phase<S, D>> + Send + Sync,
{
    factory: F,
    _marker: std::marker::PhantomData<fn(S, D)>,
}

impl<S, D, F> ClosurePhaseFactory<S, D, F>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    F: Fn() -> Box<dyn Phase<S, D>> + Send + Sync,
{
    /// Creates a new factory from a closure.
    ///
    /// The closure will be called each time a new phase is needed.
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, D, F> SolverPhaseFactory<S, D> for ClosurePhaseFactory<S, D, F>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    F: Fn() -> Box<dyn Phase<S, D>> + Send + Sync,
{
    fn create_phase(&self) -> Box<dyn Phase<S, D>> {
        (self.factory)()
    }
}
