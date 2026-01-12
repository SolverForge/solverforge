//! Score calculation count termination.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when a maximum number of score calculations is reached.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::ScoreCalculationCountTermination;
/// use solverforge_core::score::SimpleScore;
/// use solverforge_core::domain::PlanningSolution;
///
/// #[derive(Clone)]
/// struct MySolution;
/// impl PlanningSolution for MySolution {
///     type Score = SimpleScore;
///     fn score(&self) -> Option<Self::Score> { None }
///     fn set_score(&mut self, _: Option<Self::Score>) {}
/// }
///
/// // Terminate after 10,000 score calculations
/// let termination = ScoreCalculationCountTermination::<MySolution>::new(10_000);
/// ```
#[derive(Clone)]
pub struct ScoreCalculationCountTermination<S: PlanningSolution> {
    limit: u64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for ScoreCalculationCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoreCalculationCountTermination")
            .field("limit", &self.limit)
            .finish()
    }
}

impl<S: PlanningSolution> ScoreCalculationCountTermination<S> {
    /// Creates a new score calculation count termination.
    ///
    /// # Arguments
    /// * `limit` - Maximum score calculations before terminating
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D>
    for ScoreCalculationCountTermination<S>
{
    fn is_terminated(&self, solver_scope: &SolverScope<'_, S, D>) -> bool {
        solver_scope.stats().score_calculations >= self.limit
    }
}
