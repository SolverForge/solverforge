//! Score-based termination conditions.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when best score reaches or exceeds a target.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::BestScoreTermination;
/// use solverforge_core::score::SimpleScore;
///
/// let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));
/// ```
#[derive(Debug, Clone)]
pub struct BestScoreTermination<Sc: Score> {
    target_score: Sc,
}

impl<Sc: Score> BestScoreTermination<Sc> {
    pub fn new(target_score: Sc) -> Self {
        Self { target_score }
    }
}

impl<S, Sc, D> Termination<S, D> for BestScoreTermination<Sc>
where
    S: PlanningSolution<Score = Sc>,
    Sc: Score,
    D: ScoreDirector<S>,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        solver_scope
            .best_score()
            .map(|score| *score >= self.target_score)
            .unwrap_or(false)
    }
}

/// Terminates when best score becomes feasible.
pub struct BestScoreFeasibleTermination<S, F>
where
    S: PlanningSolution,
    F: Fn(&S::Score) -> bool + Send + Sync,
{
    feasibility_check: F,
    _phantom: std::marker::PhantomData<S>,
}

impl<S, F> Debug for BestScoreFeasibleTermination<S, F>
where
    S: PlanningSolution,
    F: Fn(&S::Score) -> bool + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BestScoreFeasibleTermination").finish()
    }
}

impl<S, F> BestScoreFeasibleTermination<S, F>
where
    S: PlanningSolution,
    F: Fn(&S::Score) -> bool + Send + Sync,
{
    pub fn new(feasibility_check: F) -> Self {
        Self {
            feasibility_check,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: PlanningSolution> BestScoreFeasibleTermination<S, fn(&S::Score) -> bool> {
    pub fn score_at_least_zero() -> Self {
        Self::new(|score| *score >= S::Score::zero())
    }
}

impl<S, F, D> Termination<S, D> for BestScoreFeasibleTermination<S, F>
where
    S: PlanningSolution,
    F: Fn(&S::Score) -> bool + Send + Sync,
    D: ScoreDirector<S>,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        solver_scope
            .best_score()
            .map(|score| (self.feasibility_check)(score))
            .unwrap_or(false)
    }
}
