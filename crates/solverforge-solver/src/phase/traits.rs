//! Phase trait for solver phases.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::SolverScope;

/// A solver phase that modifies the working solution.
///
/// Phases are the main building blocks of solving:
/// - Construction: Builds an initial solution
/// - LocalSearch: Iteratively improves the solution
/// - ExhaustiveSearch: Explores the solution space systematically
/// - PartitionedSearch: Parallel solving via partitioning
/// - VND: Variable Neighborhood Descent
pub trait Phase<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Executes the phase on the solver scope.
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D>);

    /// Returns the phase type name for logging/debugging.
    fn phase_type_name(&self) -> &'static str;
}
