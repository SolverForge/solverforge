//! Solver phases for different solving strategies
//!
//! Phases are the main building blocks of solving:
//! - ConstructionHeuristicPhase: Builds an initial solution
//! - LocalSearchPhase: Improves an existing solution
//! - ExhaustiveSearchPhase: Explores entire solution space
//! - PartitionedSearchPhase: Parallel solving via partitioning
//! - VndPhase: Variable Neighborhood Descent
//! - BasicConstructionPhase/BasicLocalSearchPhase: For basic variable problems

pub mod basic;
pub mod construction;
pub mod exhaustive;
pub mod localsearch;
pub mod partitioned;
pub mod vnd;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Phases are executed in sequence by the solver. Each phase has its own
/// strategy for exploring or constructing solutions.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `C` - The constraint set type
pub trait Phase<S, C>: Send + Debug
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Executes this phase.
    ///
    /// The phase should modify the working solution in the solver scope
    /// and update the best solution when improvements are found.
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, C>);

    /// Returns the name of this phase type.
    fn phase_type_name(&self) -> &'static str;
}

/// Unit type implements Phase as a no-op (empty phase list).
impl<S, C> Phase<S, C> for ()
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    fn solve(&mut self, _solver_scope: &mut SolverScope<'_, S, C>) {
        // No-op: empty phase list does nothing
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOp"
    }
}

// ((), P1)
impl<S, C, P1> Phase<S, C> for ((), P1)
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    P1: Phase<S, C>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, C>) {
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        self.1.phase_type_name()
    }
}

// (((), P1), P2)
impl<S, C, P1, P2> Phase<S, C> for (((), P1), P2)
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    P1: Phase<S, C>,
    P2: Phase<S, C>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, C>) {
        (self.0).1.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}

// ((((), P1), P2), P3)
impl<S, C, P1, P2, P3> Phase<S, C> for ((((), P1), P2), P3)
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    P1: Phase<S, C>,
    P2: Phase<S, C>,
    P3: Phase<S, C>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, C>) {
        ((self.0).0).1.solve(solver_scope);
        (self.0).1.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}

// (((((), P1), P2), P3), P4)
impl<S, C, P1, P2, P3, P4> Phase<S, C> for (((((), P1), P2), P3), P4)
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    P1: Phase<S, C>,
    P2: Phase<S, C>,
    P3: Phase<S, C>,
    P4: Phase<S, C>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, C>) {
        (((self.0).0).0).1.solve(solver_scope);
        ((self.0).0).1.solve(solver_scope);
        (self.0).1.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}
