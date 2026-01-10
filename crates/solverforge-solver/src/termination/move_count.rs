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
/// let termination = MoveCountTermination::<MySolution>::new(100_000);
/// ```
#[derive(Clone)]
pub struct MoveCountTermination<S: PlanningSolution> {
    move_count_limit: u64,
    _phantom: PhantomData<fn() -> S>,
}

impl<S: PlanningSolution> Debug for MoveCountTermination<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MoveCountTermination")
            .field("move_count_limit", &self.move_count_limit)
            .finish()
    }
}

impl<S: PlanningSolution> MoveCountTermination<S> {
    pub fn new(move_count_limit: u64) -> Self {
        Self {
            move_count_limit,
            _phantom: PhantomData,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for MoveCountTermination<S> {
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool {
        if let Some(stats) = solver_scope.statistics() {
            stats.current_moves_evaluated() >= self.move_count_limit
        } else {
            false
        }
    }
}
