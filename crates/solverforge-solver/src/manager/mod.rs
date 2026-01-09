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
//! # Example
//!
//! ```
//! use solverforge_solver::manager::{SolverManager, LocalSearchType};
//! use solverforge_core::domain::PlanningSolution;
//! use solverforge_core::score::SimpleScore;
//! use std::time::Duration;
//!
//! #[derive(Clone)]
//! struct Schedule { score: Option<SimpleScore> }
//!
//! impl PlanningSolution for Schedule {
//!     type Score = SimpleScore;
//!     fn score(&self) -> Option<Self::Score> { self.score }
//!     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
//! }
//!
//! // Create a manager with score calculator and termination
//! let manager = SolverManager::<Schedule>::builder(|_| SimpleScore::of(0))
//!     .with_construction_heuristic()
//!     .with_local_search(LocalSearchType::HillClimbing)
//!     .with_time_limit(Duration::from_secs(30))
//!     .build()
//!     .expect("Failed to build manager");
//!
//! // Create a solver from the manager
//! let solver = manager.create_solver();
//! ```

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
pub use phase_factory::{ConstructionPhaseFactory, KOptPhaseBuilder, ListConstructionPhaseBuilder, LocalSearchPhaseFactory};
pub use solver_manager::SolverManager;

use solverforge_core::domain::PlanningSolution;

use crate::phase::Phase;

/// Factory trait for creating phases.
///
/// Phase factories allow creating fresh phase instances for each solve,
/// ensuring clean state between solves. This is essential because phases
/// often maintain internal state (like step counters or tabu lists) that
/// must be reset for each new solve.
///
/// # Implementing SolverPhaseFactory
///
/// There are two common ways to implement this trait:
///
/// 1. Use [`CloneablePhaseFactory`] for phases that implement `Clone`
/// 2. Use [`ClosurePhaseFactory`] with a closure that creates phases
///
/// # Example
///
/// ```no_run
/// use solverforge_solver::manager::SolverPhaseFactory;
/// use solverforge_solver::phase::Phase;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// # #[derive(Clone)]
/// # struct MySolution { score: Option<SimpleScore> }
/// # impl PlanningSolution for MySolution {
/// #     type Score = SimpleScore;
/// #     fn score(&self) -> Option<Self::Score> { self.score }
/// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// # }
/// struct MyPhaseFactory;
///
/// impl SolverPhaseFactory<MySolution> for MyPhaseFactory {
///     fn create_phase(&self) -> Box<dyn Phase<MySolution>> {
///         // Create and return a new phase instance
///         todo!("Create phase here")
///     }
/// }
/// ```
pub trait SolverPhaseFactory<S: PlanningSolution>: Send + Sync {
    /// Creates a new phase instance.
    ///
    /// This method is called once per solve to create a fresh phase
    /// with clean state.
    fn create_phase(&self) -> Box<dyn Phase<S>>;
}

/// A simple phase factory that clones a prototype phase.
///
/// This factory stores a prototype phase and clones it for each solve.
/// Use this when your phase type implements `Clone`.
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{CloneablePhaseFactory, SolverPhaseFactory};
/// use solverforge_solver::phase::Phase;
/// use solverforge_solver::scope::SolverScope;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct MySolution { score: Option<SimpleScore> }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// // A simple cloneable phase
/// #[derive(Clone, Debug)]
/// struct SimplePhase;
///
/// impl Phase<MySolution> for SimplePhase {
///     fn solve(&mut self, _scope: &mut SolverScope<MySolution>) {}
///     fn phase_type_name(&self) -> &'static str { "SimplePhase" }
/// }
///
/// // Create factory that will clone this phase for each solve
/// let factory = CloneablePhaseFactory::new(SimplePhase);
/// let phase = factory.create_phase();
/// assert_eq!(phase.phase_type_name(), "SimplePhase");
/// ```
pub struct CloneablePhaseFactory<P> {
    prototype: P,
}

impl<P: Clone> CloneablePhaseFactory<P> {
    /// Creates a new factory from a prototype phase.
    ///
    /// The prototype will be cloned each time [`SolverPhaseFactory::create_phase()`]
    /// is called.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::CloneablePhaseFactory;
    /// use solverforge_solver::phase::Phase;
    /// use solverforge_solver::scope::SolverScope;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)]
    /// # struct S { score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// #[derive(Clone, Debug)]
    /// struct MyPhase;
    /// impl Phase<S> for MyPhase {
    ///     fn solve(&mut self, _: &mut SolverScope<S>) {}
    ///     fn phase_type_name(&self) -> &'static str { "MyPhase" }
    /// }
    ///
    /// let factory = CloneablePhaseFactory::new(MyPhase);
    /// ```
    pub fn new(prototype: P) -> Self {
        Self { prototype }
    }
}

impl<S, P> SolverPhaseFactory<S> for CloneablePhaseFactory<P>
where
    S: PlanningSolution,
    P: Phase<S> + Clone + Send + Sync + 'static,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        Box::new(self.prototype.clone())
    }
}

/// A phase factory using a closure.
///
/// This factory uses a closure to create phase instances. This is useful
/// when the phase creation requires complex logic or external dependencies
/// that cannot be easily cloned.
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::{ClosurePhaseFactory, SolverPhaseFactory};
/// use solverforge_solver::phase::Phase;
/// use solverforge_solver::scope::SolverScope;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct MySolution { score: Option<SimpleScore> }
///
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// #[derive(Debug)]
/// struct DynamicPhase { step_count: u32 }
/// impl Phase<MySolution> for DynamicPhase {
///     fn solve(&mut self, _: &mut SolverScope<MySolution>) {}
///     fn phase_type_name(&self) -> &'static str { "DynamicPhase" }
/// }
///
/// // Factory creates fresh instances with reset state
/// let factory = ClosurePhaseFactory::<MySolution, _>::new(|| {
///     Box::new(DynamicPhase { step_count: 0 })
/// });
///
/// let phase = factory.create_phase();
/// assert_eq!(phase.phase_type_name(), "DynamicPhase");
/// ```
pub struct ClosurePhaseFactory<S, F>
where
    S: PlanningSolution,
    F: Fn() -> Box<dyn Phase<S>> + Send + Sync,
{
    factory: F,
    _marker: std::marker::PhantomData<S>,
}

impl<S, F> ClosurePhaseFactory<S, F>
where
    S: PlanningSolution,
    F: Fn() -> Box<dyn Phase<S>> + Send + Sync,
{
    /// Creates a new factory from a closure.
    ///
    /// The closure will be called each time a new phase is needed.
    ///
    /// # Example
    ///
    /// ```
    /// use solverforge_solver::manager::ClosurePhaseFactory;
    /// use solverforge_solver::phase::Phase;
    /// use solverforge_solver::scope::SolverScope;
    /// use solverforge_core::domain::PlanningSolution;
    /// use solverforge_core::score::SimpleScore;
    ///
    /// # #[derive(Clone)]
    /// # struct S { score: Option<SimpleScore> }
    /// # impl PlanningSolution for S {
    /// #     type Score = SimpleScore;
    /// #     fn score(&self) -> Option<Self::Score> { self.score }
    /// #     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
    /// # }
    /// #[derive(Debug)]
    /// struct P;
    /// impl Phase<S> for P {
    ///     fn solve(&mut self, _: &mut SolverScope<S>) {}
    ///     fn phase_type_name(&self) -> &'static str { "P" }
    /// }
    ///
    /// let factory = ClosurePhaseFactory::<S, _>::new(|| Box::new(P));
    /// ```
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<S, F> SolverPhaseFactory<S> for ClosurePhaseFactory<S, F>
where
    S: PlanningSolution,
    F: Fn() -> Box<dyn Phase<S>> + Send + Sync,
{
    fn create_phase(&self) -> Box<dyn Phase<S>> {
        (self.factory)()
    }
}
