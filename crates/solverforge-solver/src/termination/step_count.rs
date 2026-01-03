//! Step count termination.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates after a step count.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::StepCountTermination;
///
/// // Terminate after 1000 steps
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

impl<S: PlanningSolution> Termination<S> for StepCountTermination {
    fn is_terminated(&self, solver_scope: &SolverScope<S>) -> bool {
        solver_scope.total_step_count() >= self.limit
    }
}
