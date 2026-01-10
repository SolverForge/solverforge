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
/// let termination = ScoreCalculationCountTermination::<MySolution>::new(10_000);
/// ```
#[derive(Clone)]
pub struct ScoreCalculationCountTermination<S: PlanningSolution> {
    score_calculation_count_limit: u64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for ScoreCalculationCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScoreCalculationCountTermination")
            .field(
                "score_calculation_count_limit",
                &self.score_calculation_count_limit,
            )
            .finish()
    }
}

impl<S: PlanningSolution> ScoreCalculationCountTermination<S> {
    pub fn new(score_calculation_count_limit: u64) -> Self {
        Self {
            score_calculation_count_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D>
    for ScoreCalculationCountTermination<S>
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        if let Some(stats) = solver_scope.statistics() {
            stats.current_score_calculations() >= self.score_calculation_count_limit
        } else {
            false
        }
    }
}
