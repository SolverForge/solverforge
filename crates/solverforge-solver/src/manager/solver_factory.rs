//! SolverFactory with zero-erasure design.
//!
//! Low-level solver infrastructure for building and executing solve phases.
//! For async job management, see [`SolverManager`](super::SolverManager).

use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::solver::NoTermination;
use crate::termination::Termination;

use super::builder::SolverFactoryBuilder;

/// Zero-erasure solver factory.
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
pub struct SolverFactory<S, D, C, P, T> {
    score_calculator: C,
    phases: P,
    termination: T,
    _marker: PhantomData<fn(S, D, P, T)>,
}

impl<S, D, C, P, T> SolverFactory<S, D, C, P, T>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
    P: Phase<S, D>,
    T: Termination<S, D>,
{
    /// Creates a new SolverFactory with concrete types.
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

    /// Creates a Solver from this factory's configuration.
    ///
    /// The returned Solver can be used with `solve_with_director()` for
    /// macro-generated code or `solve()` for direct use.
    pub fn create_solver(self) -> crate::solver::Solver<P, Option<T>, S, ()> {
        crate::solver::Solver::new(self.phases).with_termination(self.termination)
    }
}

/// Creates a builder for SolverFactory.
///
/// Use `SolverFactoryBuilder::new()` directly for full type control.
pub fn solver_factory_builder<S, D, C>(
    score_calculator: C,
) -> SolverFactoryBuilder<S, D, C, (), NoTermination>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    SolverFactoryBuilder::new(score_calculator)
}

impl<S: PlanningSolution> SolverFactory<S, (), (), (), ()> {
    /// Creates a new builder for SolverFactory.
    ///
    /// This allows calling `SolverFactory::<MySolution>::builder(score_fn)`
    /// to start building a solver factory.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let factory = SolverFactory::<MySolution>::builder(|s| calculate_score(s))
    ///     .with_phase(construction_phase)
    ///     .with_phase(local_search_phase)
    ///     .with_step_limit(1000)
    ///     .build();
    /// ```
    pub fn builder<D, C>(score_calculator: C) -> SolverFactoryBuilder<S, D, C, (), NoTermination>
    where
        C: Fn(&S) -> S::Score + Send + Sync,
    {
        SolverFactoryBuilder::new(score_calculator)
    }
}
