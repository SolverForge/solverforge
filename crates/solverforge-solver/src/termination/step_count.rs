//! Step count termination.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

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

impl<S, C> Termination<S, C> for StepCountTermination
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, C>) -> bool {
        solver_scope.total_step_count() >= self.limit
    }
}
