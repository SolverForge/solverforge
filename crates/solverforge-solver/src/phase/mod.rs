//! Solver phases for different solving strategies
//!
//! Phases are the main building blocks of solving:
//! - ConstructionHeuristicPhase: Builds an initial solution
//! - LocalSearchPhase: Improves an existing solution
//! - ExhaustiveSearchPhase: Explores entire solution space
//! - PartitionedSearchPhase: Parallel solving via partitioning
//! - VndPhase: Variable Neighborhood Descent

pub mod construction;
pub mod exhaustive;
pub mod fluent;
pub mod localsearch;
pub mod partitioned;
pub mod vnd;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Phases are executed in sequence by the solver. Each phase has its own
/// strategy for exploring or constructing solutions.
pub trait Phase<S: PlanningSolution>: Send + Debug {
    /// Executes this phase.
    ///
    /// The phase should modify the working solution in the solver scope
    /// and update the best solution when improvements are found.
    fn solve(&mut self, solver_scope: &mut SolverScope<S>);

    /// Returns the name of this phase type.
    fn phase_type_name(&self) -> &'static str;
}
