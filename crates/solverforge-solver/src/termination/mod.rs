//! Termination conditions for solver phases.

mod best_score;
mod composite;
mod diminished_returns;
mod external;
mod move_count;
mod score_calculation_count;
mod step_count;
mod time;
mod unimproved;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::SolverScope;

pub use best_score::{BestScoreFeasibleTermination, BestScoreTermination};
pub use composite::{AndTermination, OrTermination};
pub use diminished_returns::DiminishedReturnsTermination;
pub use external::ExternalTermination;
pub use move_count::MoveCountTermination;
pub use score_calculation_count::ScoreCalculationCountTermination;
pub use step_count::StepCountTermination;
pub use time::TimeTermination;
pub use unimproved::{UnimprovedStepCountTermination, UnimprovedTimeTermination};

/// Trait for determining when to stop solving.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
pub trait Termination<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Returns true if solving should terminate.
    fn is_terminated(&self, solver_scope: &SolverScope<S, D>) -> bool;
}

#[cfg(test)]
mod tests;
