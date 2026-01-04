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

use crate::scope::SolverScope;

pub use best_score::{BestScoreFeasibleTermination, BestScoreTermination};
pub use composite::{AndCompositeTermination, OrCompositeTermination};
pub use diminished_returns::DiminishedReturnsTermination;
pub use move_count::MoveCountTermination;
pub use score_calculation_count::ScoreCalculationCountTermination;
pub use step_count::StepCountTermination;
pub use time::TimeTermination;
pub use unimproved::{UnimprovedStepCountTermination, UnimprovedTimeTermination};

/// Trait for determining when to stop solving.
pub trait Termination<S: PlanningSolution>: Send + Debug {
    /// Returns true if solving should terminate.
    fn is_terminated(&self, solver_scope: &SolverScope<S>) -> bool;
}

#[cfg(test)]
mod tests;
