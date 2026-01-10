//! High-level solver management.

mod builder;
mod config;
mod phase_factory;
mod solver_manager;

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
pub use phase_factory::{
    BasicConstructionPhaseBuilder, BasicLocalSearchPhaseBuilder, ConstructionPhaseFactory,
    KOptPhaseBuilder, ListConstructionPhaseBuilder, LocalSearchPhaseFactory,
};
pub use solver_manager::SolverManager;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;

/// Factory trait for creating phases - generic over phase type for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `P` - The concrete phase type produced by this factory
pub trait SolverPhaseFactory<S, D, P>: Send + Sync
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
{
    /// Creates a new phase instance.
    fn create_phase(&self) -> P;
}

/// A simple phase factory that clones a prototype phase.
pub struct CloneablePhaseFactory<P> {
    prototype: P,
}

impl<P: Clone> CloneablePhaseFactory<P> {
    pub fn new(prototype: P) -> Self {
        Self { prototype }
    }
}

impl<S, D, P> SolverPhaseFactory<S, D, P> for CloneablePhaseFactory<P>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D> + Clone + Send + Sync,
{
    fn create_phase(&self) -> P {
        self.prototype.clone()
    }
}

/// A phase factory using a closure.
pub struct ClosurePhaseFactory<P, F> {
    factory: F,
    _marker: std::marker::PhantomData<P>,
}

impl<P, F> ClosurePhaseFactory<P, F> {
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, D, P, F> SolverPhaseFactory<S, D, P> for ClosurePhaseFactory<P, F>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
    F: Fn() -> P + Send + Sync,
{
    fn create_phase(&self) -> P {
        (self.factory)()
    }
}
