//! SolverManager implementation.

use solverforge_core::domain::PlanningSolution;

use crate::phase::Phase;
use crate::solver::Solver;
use crate::termination::Termination;

use super::SolverPhaseFactory;

/// High-level solver manager that creates configured solvers.
///
/// # Example
///
/// ```
/// use solverforge_solver::manager::SolverManager;
/// use solverforge_core::domain::PlanningSolution;
/// use solverforge_core::score::SimpleScore;
///
/// #[derive(Clone)]
/// struct Schedule { score: Option<SimpleScore> }
///
/// impl PlanningSolution for Schedule {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { self.score }
///     fn set_score(&mut self, score: Option<Self::Score>) { self.score = score; }
/// }
///
/// let manager = SolverManager::<Schedule>::builder()
///     .build()
///     .unwrap();
///
/// let solver = manager.create_solver();
/// ```
pub struct SolverManager<S: PlanningSolution> {
    phase_factories: Vec<Box<dyn SolverPhaseFactory<S>>>,
    termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S>> + Send + Sync>>,
}

impl<S: PlanningSolution> SolverManager<S> {
    /// Creates a new builder.
    pub fn builder() -> super::SolverManagerBuilder<S> {
        super::SolverManagerBuilder::new()
    }

    pub(crate) fn new(
        phase_factories: Vec<Box<dyn SolverPhaseFactory<S>>>,
        termination_factory: Option<Box<dyn Fn() -> Box<dyn Termination<S>> + Send + Sync>>,
    ) -> Self {
        Self {
            phase_factories,
            termination_factory,
        }
    }

    /// Creates a fresh [`Solver`] instance with configured phases.
    pub fn create_solver(&self) -> Solver<S> {
        let phases: Vec<Box<dyn Phase<S>>> = self
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
