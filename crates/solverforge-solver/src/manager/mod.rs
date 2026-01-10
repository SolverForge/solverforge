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

/// Factory trait for creating phases.
pub trait SolverPhaseFactory<S: PlanningSolution, D: ScoreDirector<S>>: Send + Sync {
    fn create_phase(&self) -> Box<dyn Phase<S, D>>;
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
pub struct ClosurePhaseFactory<S, F>
where
    S: PlanningSolution,
{
    factory: F,
    _marker: std::marker::PhantomData<S>,
}

impl<S, F> ClosurePhaseFactory<S, F>
where
    S: PlanningSolution,
{
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, D, F> SolverPhaseFactory<S, D> for ClosurePhaseFactory<S, F>
where
    S: PlanningSolution,
    D: ScoreDirector<S> + 'static,
    F: Fn() -> Box<dyn Phase<S, D>> + Send + Sync,
{
    fn create_phase(&self) -> Box<dyn Phase<S, D>> {
        (self.factory)()
    }
}
