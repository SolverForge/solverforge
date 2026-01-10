//! SolverManager implementation.

#![allow(clippy::type_complexity)]

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::termination::Termination;

use super::SolverPhaseFactory;

/// High-level solver manager for runtime configuration.
///
/// `SolverManager` stores solver configuration and can create phases on demand.
/// Uses `Box<dyn Phase<S, D>>` for runtime configuration from TOML/YAML files.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
/// * `C` - The score calculator type (defaults to function pointer)
///
/// # Zero-Erasure Design
///
/// The score calculator is stored as a concrete generic type parameter `C`,
/// not as `Arc<dyn Fn>`. This eliminates virtual dispatch overhead for the
/// hot path (score calculation is called millions of times per solve).
///
/// The default `C = fn(&S) -> S::Score` allows writing `SolverManager::<S, D>::builder(...)`
/// without specifying the calculator type (it's inferred from the builder).
pub struct SolverManager<S, D, C = fn(&S) -> <S as PlanningSolution>::Score>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Score calculator function (zero-erasure: concrete generic type).
    score_calculator: C,

    /// Configured phases (as factories that create fresh phases per solve).
    phase_factories: Vec<Box<dyn SolverPhaseFactory<S, D>>>,

    /// Global termination condition factory.
    termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S, D>> + Send + Sync>>,

    /// Phantom data for D.
    _marker: PhantomData<D>,
}

impl<S, D> SolverManager<S, D, fn(&S) -> S::Score>
where
    S: PlanningSolution,
    D: ScoreDirector<S> + 'static,
{
    /// Creates a new [`SolverManagerBuilder`](super::SolverManagerBuilder) with the given score calculator.
    ///
    /// The score calculator is a function that computes the score for a solution.
    /// This is the entry point for building a `SolverManager`.
    pub fn builder<F>(score_calculator: F) -> super::SolverManagerBuilder<S, D, F>
    where
        F: Fn(&S) -> S::Score + Send + Sync + 'static,
    {
        super::SolverManagerBuilder::new(score_calculator)
    }
}

impl<S, D, C> SolverManager<S, D, C>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a SolverManager with explicit configuration (zero-erasure).
    pub(crate) fn new(
        score_calculator: C,
        phase_factories: Vec<Box<dyn SolverPhaseFactory<S, D>>>,
        termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S, D>> + Send + Sync>>,
    ) -> Self {
        Self {
            score_calculator,
            phase_factories,
            termination_factory,
            _marker: PhantomData,
        }
    }

    /// Creates fresh phases from the configured factories.
    ///
    /// Each call returns new phases with clean state, suitable for a new solve.
    pub fn create_phases(&self) -> Vec<Box<dyn Phase<S, D>>> {
        self.phase_factories.iter().map(|f| f.create_phase()).collect()
    }

    /// Creates a fresh termination condition if configured.
    pub fn create_termination(&self) -> Option<Box<dyn Termination<S, D>>> {
        self.termination_factory.as_ref().map(|f| f())
    }

    /// Returns a reference to the score calculator function.
    pub fn score_calculator(&self) -> &C {
        &self.score_calculator
    }

    /// Calculates the score for a solution using the configured calculator.
    pub fn calculate_score(&self, solution: &S) -> S::Score {
        (self.score_calculator)(solution)
    }
}
