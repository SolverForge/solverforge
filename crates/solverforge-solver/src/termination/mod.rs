//! Termination conditions for solver phases.

mod best_score;
mod composite;
mod diminished_returns;
mod move_count;
mod score_calculation_count;
mod step_count;
mod time;
mod unimproved;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use crate::scope::SolverScope;

pub use best_score::{BestScoreFeasibleTermination, BestScoreTermination};
pub use composite::{AndTermination, OrTermination};
pub use diminished_returns::DiminishedReturnsTermination;
pub use move_count::MoveCountTermination;
pub use score_calculation_count::ScoreCalculationCountTermination;
pub use step_count::StepCountTermination;
pub use time::TimeTermination;
pub use unimproved::{UnimprovedStepCountTermination, UnimprovedTimeTermination};

/// Trait for determining when to stop solving.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `C` - The constraint set type
pub trait Termination<S, C>: Send + Debug
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Returns true if solving should terminate.
    fn is_terminated(&self, solver_scope: &SolverScope<S, C>) -> bool;
}

#[cfg(test)]
mod tests;
