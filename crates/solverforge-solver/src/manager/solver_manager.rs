//! SolverManager implementation.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::solver::Solver;
use crate::termination::Termination;

/// High-level solver manager that creates configured solvers.
///
/// Generic over the phase type and termination type for zero-erasure.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `P` - The phase type
/// * `T` - The termination type
/// * `PF` - The phase factory closure type
/// * `TF` - The termination factory closure type
pub struct SolverManager<S, D, P, T, PF, TF>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    phase_factory: PF,
    termination_factory: Option<TF>,
    _marker: PhantomData<(S, D, P, T)>,
}

impl<S, D, P, T, PF, TF> SolverManager<S, D, P, T, PF, TF>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
    T: Termination<S, D>,
    PF: Fn() -> P + Send + Sync,
    TF: Fn() -> T + Send + Sync,
{
    /// Creates a new solver manager with phase and termination factories.
    pub fn new(phase_factory: PF, termination_factory: TF) -> Self {
        Self {
            phase_factory,
            termination_factory: Some(termination_factory),
            _marker: PhantomData,
        }
    }

    /// Creates a solver with the configured phase and termination.
    pub fn create_solver(&self) -> Solver<S, D, P, T> {
        let phase = (self.phase_factory)();
        let termination = self.termination_factory.as_ref().map(|f| f());
        Solver::new(phase, termination)
    }
}

impl<S, D, P, PF> SolverManager<S, D, P, (), PF, fn() -> ()>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
    PF: Fn() -> P + Send + Sync,
{
    /// Creates a solver manager with only a phase factory (no termination).
    pub fn with_phase(phase_factory: PF) -> Self {
        Self {
            phase_factory,
            termination_factory: None,
            _marker: PhantomData,
        }
    }
}
