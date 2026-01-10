//! Score-based termination conditions.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when the best score reaches or exceeds a target score.
///
/// This is useful when you know what score you're aiming for (e.g., a perfect
/// score of 0 for constraint satisfaction problems).
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::BestScoreTermination;
/// use solverforge_core::score::SimpleScore;
///
/// // Terminate when score reaches 0 (no constraint violations)
/// let term: BestScoreTermination<SimpleScore> = BestScoreTermination::new(SimpleScore::of(0));
/// ```
#[derive(Debug, Clone)]
pub struct BestScoreTermination<Sc: Score> {
    target_score: Sc,
}

impl<Sc: Score> BestScoreTermination<Sc> {
    /// Creates a termination that stops when best score >= target.
    pub fn new(target_score: Sc) -> Self {
        Self { target_score }
    }
}

impl<S, D, Sc> Termination<S, D> for BestScoreTermination<Sc>
where
    S: PlanningSolution<Score = Sc>,
    D: ScoreDirector<S>,
    Sc: Score,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        solver_scope
            .best_score()
            .map(|score| *score >= self.target_score)
            .unwrap_or(false)
    }
}

/// Terminates when the best score becomes feasible.
///
/// A score is considered feasible when it meets a feasibility check defined
/// by a user-provided function. For HardSoftScore, this typically means
/// hard score >= 0 (no hard constraint violations).
///
/// # Zero-Erasure Design
///
/// The feasibility check function `F` is stored as a concrete generic type
/// parameter, eliminating virtual dispatch overhead when checking termination.
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
    /// Creates a termination with a custom feasibility check.
    pub fn new(feasibility_check: F) -> Self {
        Self {
            feasibility_check,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: PlanningSolution> BestScoreFeasibleTermination<S, fn(&S::Score) -> bool> {
    /// Creates a termination that checks if score >= zero.
    ///
    /// This is the typical feasibility check for most score types.
    pub fn score_at_least_zero() -> Self {
        Self::new(|score| *score >= S::Score::zero())
    }
}

impl<S, D, F> Termination<S, D> for BestScoreFeasibleTermination<S, F>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    F: Fn(&S::Score) -> bool + Send + Sync,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        solver_scope
            .best_score()
            .map(|score| (self.feasibility_check)(score))
            .unwrap_or(false)
    }
}
