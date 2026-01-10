//! Solver phases for different solving strategies.

pub mod construction;
pub mod exhaustive;
pub mod localsearch;
pub mod partitioned;
pub mod vnd;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Generic over `D: ScoreDirector<S>` for zero type erasure.
pub trait Phase<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Executes this phase.
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>);

    /// Returns the name of this phase type.
    fn phase_type_name(&self) -> &'static str;
}
