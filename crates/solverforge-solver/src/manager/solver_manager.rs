//! SolverManager with zero-erasure design.

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::solver::NoTermination;
use crate::termination::Termination;

use super::SolverManagerBuilder;

/// Zero-erasure solver manager.
///
/// Stores phases as a concrete tuple type `P`, score calculator as `C`,
/// and termination as `T`. No dynamic dispatch anywhere.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
/// * `C` - The score calculator type
/// * `P` - The phases tuple type
/// * `T` - The termination type
pub struct SolverManager<S, D, C, P, T>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
    P: Phase<S, D>,
    T: Termination<S, D>,
{
    score_calculator: C,
    phases: P,
    termination: T,
    _marker: PhantomData<fn(S, D)>,
}

impl<S, D, C, P, T> SolverManager<S, D, C, P, T>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
    P: Phase<S, D>,
    T: Termination<S, D>,
{
    /// Creates a new SolverManager with concrete types.
    pub fn new(score_calculator: C, phases: P, termination: T) -> Self {
        Self {
            score_calculator,
            phases,
            termination,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to the score calculator.
    pub fn score_calculator(&self) -> &C {
        &self.score_calculator
    }

    /// Calculates score for a solution.
    pub fn calculate_score(&self, solution: &S) -> S::Score {
        (self.score_calculator)(solution)
    }

    /// Returns a reference to the phases.
    pub fn phases(&self) -> &P {
        &self.phases
    }

    /// Returns a mutable reference to the phases.
    pub fn phases_mut(&mut self) -> &mut P {
        &mut self.phases
    }

    /// Returns a reference to the termination.
    pub fn termination(&self) -> &T {
        &self.termination
    }

    /// Solves using the configured phases and termination.
    pub fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
        solver_scope.start_solving();
        self.phases.solve(solver_scope);
    }
}

/// Creates a builder for SolverManager.
///
/// Use `SolverManagerBuilder::new()` directly for full type control.
pub fn solver_manager_builder<S, D, C>(
    score_calculator: C,
) -> SolverManagerBuilder<S, D, C, (), NoTermination>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    SolverManagerBuilder::new(score_calculator)
}
