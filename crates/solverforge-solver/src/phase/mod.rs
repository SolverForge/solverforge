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
pub mod vnd;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::scope::SolverScope;

/// A phase of the solving process.
///
/// Phases are executed in sequence by the solver. Each phase has its own
/// strategy for exploring or constructing solutions.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
pub trait Phase<S: PlanningSolution, D: ScoreDirector<S>>: Send + Debug {
    /// Executes this phase.
    ///
    /// The phase should modify the working solution in the solver scope
    /// and update the best solution when improvements are found.
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>);

    /// Returns the name of this phase type.
    fn phase_type_name(&self) -> &'static str;
}

/// Unit type implements Phase as a no-op (empty phase list).
impl<S: PlanningSolution, D: ScoreDirector<S>> Phase<S, D> for () {
    fn solve(&mut self, _solver_scope: &mut SolverScope<S, D>) {
        // No-op: empty phase list does nothing
    }

    fn phase_type_name(&self) -> &'static str {
        "NoOp"
    }
}

// ((), P1)
impl<S, D, P1> Phase<S, D> for ((), P1)
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P1: Phase<S, D>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        self.1.phase_type_name()
    }
}

// (((), P1), P2)
impl<S, D, P1, P2> Phase<S, D> for (((), P1), P2)
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P1: Phase<S, D>,
    P2: Phase<S, D>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        (self.0).1.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}

// ((((), P1), P2), P3)
impl<S, D, P1, P2, P3> Phase<S, D> for ((((), P1), P2), P3)
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P1: Phase<S, D>,
    P2: Phase<S, D>,
    P3: Phase<S, D>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        ((self.0).0).1.solve(solver_scope);
        (self.0).1.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}

// (((((), P1), P2), P3), P4)
impl<S, D, P1, P2, P3, P4> Phase<S, D> for (((((), P1), P2), P3), P4)
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P1: Phase<S, D>,
    P2: Phase<S, D>,
    P3: Phase<S, D>,
    P4: Phase<S, D>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        (((self.0).0).0).1.solve(solver_scope);
        ((self.0).0).1.solve(solver_scope);
        (self.0).1.solve(solver_scope);
        self.1.solve(solver_scope);
    }

    fn phase_type_name(&self) -> &'static str {
        "PhaseTuple"
    }
}
