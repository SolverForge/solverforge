//! Step count termination.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates after a step count.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::StepCountTermination;
///
/// let term = StepCountTermination::new(1000);
/// ```
#[derive(Debug, Clone)]
pub struct StepCountTermination {
    limit: u64,
}

impl StepCountTermination {
    pub fn new(limit: u64) -> Self {
        Self { limit }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for StepCountTermination {
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        solver_scope.total_step_count() >= self.limit
    }
}
