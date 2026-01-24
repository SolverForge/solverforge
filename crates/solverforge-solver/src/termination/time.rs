//! Time-based termination.

use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates after a time limit.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use solverforge_solver::termination::TimeTermination;
///
/// let term = TimeTermination::new(Duration::from_secs(30));
/// let term = TimeTermination::seconds(30);
/// let term = TimeTermination::millis(500);
/// ```
#[derive(Debug, Clone)]
pub struct TimeTermination {
    limit: Duration,
}

impl TimeTermination {
    pub fn new(limit: Duration) -> Self {
        Self { limit }
    }

    pub fn millis(ms: u64) -> Self {
        Self::new(Duration::from_millis(ms))
    }

    pub fn seconds(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}

impl<S, C> Termination<S, C> for TimeTermination
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, C>) -> bool {
        solver_scope.elapsed().is_some_and(|e| e >= self.limit)
    }
}
