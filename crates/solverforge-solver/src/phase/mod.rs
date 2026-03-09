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
pub mod localsearch;
pub mod partitioned;
mod tuple_impl;
pub mod vnd;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::BestSolutionCallback;
use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Phases are executed in sequence by the solver. Each phase has its own
/// strategy for exploring or constructing solutions.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `BestCb` - The best-solution callback type (default `()`)
pub trait Phase<S: PlanningSolution, D: ScoreDirector<S>, BestCb: BestSolutionCallback<S> = ()>:
    Send + Debug
{
    /// Executes this phase.
    ///
    /// The phase should modify the working solution in the solver scope
    /// and update the best solution when improvements are found.
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, BestCb>);

    /// Returns the name of this phase type.
    fn phase_type_name(&self) -> &'static str;
}

/// Unit type implements Phase as a no-op (empty phase list).
impl<S: PlanningSolution, D: ScoreDirector<S>, BestCb: BestSolutionCallback<S>> Phase<S, D, BestCb>
    for ()
{
    fn solve(&mut self, _solver_scope: &mut SolverScope<'_, S, D, BestCb>) {
        // No-op: empty phase list does nothing
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOp"
    }
}
