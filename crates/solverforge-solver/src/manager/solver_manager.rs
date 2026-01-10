//! SolverManager implementation.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::solver::Solver;
use crate::termination::Termination;

use super::SolverPhaseFactory;

/// High-level solver manager that creates configured solvers.
pub struct SolverManager<S: PlanningSolution, D: ScoreDirector<S>> {
    phase_factories: Vec<Box<dyn SolverPhaseFactory<S, D>>>,
    termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S, D>> + Send + Sync>>,
}

impl<S: PlanningSolution + 'static, D: ScoreDirector<S> + 'static> SolverManager<S, D> {
    pub fn builder() -> super::SolverManagerBuilder<S, D> {
        super::SolverManagerBuilder::new()
    }

    pub(crate) fn new(
        phase_factories: Vec<Box<dyn SolverPhaseFactory<S, D>>>,
        termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S, D>> + Send + Sync>>,
    ) -> Self {
        Self {
            phase_factories,
            termination_factory,
        }
    }

    pub fn create_solver(&self) -> Solver<S, D> {
        let phases: Vec<Box<dyn Phase<S, D>>> = self
            .phase_factories
            .iter()
            .map(|f| f.create_phase())
            .collect();

        let mut solver = Solver::new(phases);

        if let Some(factory) = &self.termination_factory {
            solver = solver.with_termination(factory());
        }

        solver
    }
}
