//! Solver implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::SolverScope;
use crate::termination::Termination;

/// The main solver that optimizes planning solutions.
///
/// Uses macro-generated tuple implementations for phases, preserving
/// concrete types through the entire pipeline (zero-erasure architecture).
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag reference
/// * `P` - Tuple of phases to execute
/// * `T` - Termination condition (use `Option<ConcreteTermination>`)
/// * `S` - Solution type
/// * `C` - Constraint set type
pub struct Solver<'t, P, T, S, C> {
    phases: P,
    termination: T,
    terminate: Option<&'t AtomicBool>,
    config: Option<SolverConfig>,
    time_limit: Option<Duration>,
    _phantom: PhantomData<fn(S, C)>,
}

impl<P: Debug, T: Debug, S, C> Debug for Solver<'_, P, T, S, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("phases", &self.phases)
            .field("termination", &self.termination)
            .finish()
    }
}

impl<P, S, C> Solver<'static, P, NoTermination, S, C>
where
    S: PlanningSolution,
{
    /// Creates a new solver with the given phases tuple and no termination.
    pub fn new(phases: P) -> Self {
        Solver {
            phases,
            termination: NoTermination,
            terminate: None,
            config: None,
            time_limit: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the termination condition.
    pub fn with_termination<T>(self, termination: T) -> Solver<'static, P, Option<T>, S, C> {
        Solver {
            phases: self.phases,
            termination: Some(termination),
            terminate: self.terminate,
            config: self.config,
            time_limit: self.time_limit,
            _phantom: PhantomData,
        }
    }
}

impl<'t, P, T, S, C> Solver<'t, P, T, S, C>
where
    S: PlanningSolution,
{
    /// Sets the external termination flag.
    ///
    /// The solver will check this flag periodically and terminate early if set.
    pub fn with_terminate(self, terminate: &AtomicBool) -> Solver<'_, P, T, S, C> {
        Solver {
            phases: self.phases,
            termination: self.termination,
            terminate: Some(terminate),
            config: self.config,
            time_limit: self.time_limit,
            _phantom: PhantomData,
        }
    }

    /// Sets the time limit for solving.
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    /// Sets configuration.
    pub fn with_config(mut self, config: SolverConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Returns the configuration if set.
    pub fn config(&self) -> Option<&SolverConfig> {
        self.config.as_ref()
    }
}

/// Marker type indicating no termination.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTermination;

/// Marker trait for termination types that can be used in Solver.
pub trait MaybeTermination<S, C>: Send
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    /// Checks if the solver should terminate.
    fn should_terminate(&self, solver_scope: &SolverScope<'_, S, C>) -> bool;
}

impl<S, C, T> MaybeTermination<S, C> for Option<T>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    T: Termination<S, C>,
{
    fn should_terminate(&self, solver_scope: &SolverScope<'_, S, C>) -> bool {
        match self {
            Some(t) => t.is_terminated(solver_scope),
            None => false,
        }
    }
}

impl<S, C> MaybeTermination<S, C> for NoTermination
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    fn should_terminate(&self, _solver_scope: &SolverScope<'_, S, C>) -> bool {
        false
    }
}

impl<S, C> Termination<S, C> for NoTermination
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    fn is_terminated(&self, _solver_scope: &SolverScope<'_, S, C>) -> bool {
        false
    }
}

macro_rules! impl_solver {
    ($($idx:tt: $P:ident),+) => {
        impl<'t, S, C, T, $($P),+> Solver<'t, ($($P,)+), T, S, C>
        where
            S: PlanningSolution,
            S::Score: Score,
            C: ConstraintSet<S, S::Score>,
            T: MaybeTermination<S, C>,
            $($P: Phase<S, C>,)+
        {
            /// Solves using the provided score director.
            pub fn solve(&mut self, score_director: ScoreDirector<S, C>) -> S {
                let mut solver_scope = SolverScope::with_terminate(score_director, self.terminate);
                if let Some(limit) = self.time_limit {
                    solver_scope.set_time_limit(limit);
                }
                solver_scope.start_solving();

                // Execute phases with termination checking
                $(
                    if !self.check_termination(&solver_scope) {
                        tracing::debug!(
                            "Starting phase {} ({})",
                            $idx,
                            self.phases.$idx.phase_type_name()
                        );
                        self.phases.$idx.solve(&mut solver_scope);
                        tracing::debug!(
                            "Finished phase {} ({}) with score {:?}",
                            $idx,
                            self.phases.$idx.phase_type_name(),
                            solver_scope.best_score()
                        );
                    }
                )+

                solver_scope.take_best_or_working_solution()
            }

            fn check_termination(&self, solver_scope: &SolverScope<'_, S, C>) -> bool {
                // Check external termination flag first
                if solver_scope.is_terminate_early() {
                    return true;
                }
                // Then check configured termination conditions
                self.termination.should_terminate(solver_scope)
            }
        }
    };
}

macro_rules! impl_solver_with_director {
    ($($idx:tt: $P:ident),+) => {
        impl<'t, S, T, $($P),+> Solver<'t, ($($P,)+), T, S, ()>
        where
            S: PlanningSolution,
            S::Score: Score,
            T: Send,
        {
            /// Solves using a provided score director.
            pub fn solve_with_director<C>(self, director: ScoreDirector<S, C>) -> S
            where
                C: ConstraintSet<S, S::Score>,
                T: MaybeTermination<S, C>,
                $($P: Phase<S, C>,)+
            {
                let mut solver: Solver<'t, ($($P,)+), T, S, C> = Solver {
                    phases: self.phases,
                    termination: self.termination,
                    terminate: self.terminate,
                    config: self.config,
                    time_limit: self.time_limit,
                    _phantom: PhantomData,
                };
                solver.solve(director)
            }
        }
    };
}

impl_solver_with_director!(0: P0);
impl_solver_with_director!(0: P0, 1: P1);
impl_solver_with_director!(0: P0, 1: P1, 2: P2);
impl_solver_with_director!(0: P0, 1: P1, 2: P2, 3: P3);
impl_solver_with_director!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4);
impl_solver_with_director!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5);
impl_solver_with_director!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6);
impl_solver_with_director!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6, 7: P7);

impl_solver!(0: P0);
impl_solver!(0: P0, 1: P1);
impl_solver!(0: P0, 1: P1, 2: P2);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6);
impl_solver!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6, 7: P7);
