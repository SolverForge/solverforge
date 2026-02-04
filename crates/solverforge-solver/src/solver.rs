//! Solver implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
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
/// * `D` - Score director type
#[allow(clippy::type_complexity)]
pub struct Solver<'t, P, T, S, D> {
    phases: P,
    termination: T,
    terminate: Option<&'t AtomicBool>,
    config: Option<SolverConfig>,
    time_limit: Option<Duration>,
    /// Callback invoked when a better solution is found during solving.
    best_solution_callback: Option<Box<dyn Fn(&S) + Send + Sync + 't>>,
    _phantom: PhantomData<fn(S, D)>,
}

impl<P: Debug, T: Debug, S, D> Debug for Solver<'_, P, T, S, D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("phases", &self.phases)
            .field("termination", &self.termination)
            .finish()
    }
}

impl<P, S, D> Solver<'static, P, NoTermination, S, D>
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
            best_solution_callback: None,
            _phantom: PhantomData,
        }
    }

    /// Sets the termination condition.
    pub fn with_termination<T>(self, termination: T) -> Solver<'static, P, Option<T>, S, D> {
        Solver {
            phases: self.phases,
            termination: Some(termination),
            terminate: self.terminate,
            config: self.config,
            time_limit: self.time_limit,
            best_solution_callback: self.best_solution_callback,
            _phantom: PhantomData,
        }
    }
}

impl<'t, P, T, S, D> Solver<'t, P, T, S, D>
where
    S: PlanningSolution,
{
    /// Sets the external termination flag.
    ///
    /// The solver will check this flag periodically and terminate early if set.
    pub fn with_terminate(self, terminate: &'t AtomicBool) -> Solver<'t, P, T, S, D> {
        Solver {
            phases: self.phases,
            termination: self.termination,
            terminate: Some(terminate),
            config: self.config,
            time_limit: self.time_limit,
            best_solution_callback: self.best_solution_callback,
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

    /// Sets a callback to be invoked when a better solution is found during solving.
    pub fn with_best_solution_callback(
        mut self,
        callback: Box<dyn Fn(&S) + Send + Sync + 't>,
    ) -> Self {
        self.best_solution_callback = Some(callback);
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
pub trait MaybeTermination<S: PlanningSolution, D: ScoreDirector<S>>: Send {
    /// Checks if the solver should terminate.
    fn should_terminate(&self, solver_scope: &SolverScope<'_, S, D>) -> bool;
}

impl<S: PlanningSolution, D: ScoreDirector<S>, T: Termination<S, D>> MaybeTermination<S, D>
    for Option<T>
{
    fn should_terminate(&self, solver_scope: &SolverScope<'_, S, D>) -> bool {
        match self {
            Some(t) => t.is_terminated(solver_scope),
            None => false,
        }
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> MaybeTermination<S, D> for NoTermination {
    fn should_terminate(&self, _solver_scope: &SolverScope<'_, S, D>) -> bool {
        false
    }
}

impl<S: PlanningSolution, D: ScoreDirector<S>> Termination<S, D> for NoTermination {
    fn is_terminated(&self, _solver_scope: &SolverScope<'_, S, D>) -> bool {
        false
    }
}

macro_rules! impl_solver {
    ($($idx:tt: $P:ident),+) => {
        impl<'t, S, D, T, $($P),+> Solver<'t, ($($P,)+), T, S, D>
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            T: MaybeTermination<S, D>,
            $($P: Phase<S, D>,)+
        {
            /// Solves using the provided score director.
            pub fn solve(&mut self, score_director: D) -> S {
                let mut solver_scope = SolverScope::with_terminate(score_director, self.terminate);
                if let Some(limit) = self.time_limit {
                    solver_scope.set_time_limit(limit);
                }
                if let Some(callback) = self.best_solution_callback.take() {
                    solver_scope = solver_scope.with_best_solution_callback(callback);
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

            fn check_termination(&self, solver_scope: &SolverScope<'_, S, D>) -> bool {
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
            T: Send,
        {
            /// Solves using a provided score director.
            pub fn solve_with_director<D>(self, director: D) -> S
            where
                D: ScoreDirector<S>,
                T: MaybeTermination<S, D>,
                $($P: Phase<S, D>,)+
            {
                let mut solver: Solver<'t, ($($P,)+), T, S, D> = Solver {
                    phases: self.phases,
                    termination: self.termination,
                    terminate: self.terminate,
                    config: self.config,
                    time_limit: self.time_limit,
                    best_solution_callback: self.best_solution_callback,
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
