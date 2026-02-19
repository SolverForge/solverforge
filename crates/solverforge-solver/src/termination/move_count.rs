//! Move count termination.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use super::Termination;
use crate::scope::SolverScope;

/// Terminates when a maximum number of moves have been evaluated.
///
/// # Example
///
/// ```
/// use solverforge_solver::termination::MoveCountTermination;
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
/// // Terminate after evaluating 100,000 moves
/// let termination = MoveCountTermination::<MySolution>::new(100_000);
/// ```
#[derive(Clone)]
pub struct MoveCountTermination<S: PlanningSolution> {
    limit: u64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for MoveCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveCountTermination")
            .field("limit", &self.limit)
            .finish()
    }
}

impl<S: PlanningSolution> MoveCountTermination<S> {
    /// Creates a new move count termination.
    ///
    /// # Arguments
    /// * `limit` - Maximum moves to evaluate before terminating
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for MoveCountTermination<S> {
    fn is_terminated(&self, solver_scope: &SolverScope<'_, S, D>) -> bool {
        solver_scope.stats().moves_evaluated >= self.limit
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<S, D>) {
        let limit = match solver_scope.inphase_move_count_limit {
            Some(existing) => existing.min(self.limit),
            None => self.limit,
        };
        solver_scope.inphase_move_count_limit = Some(limit);
    }
}
