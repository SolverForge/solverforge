// Solver implementation.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::manager::{SolverRuntime, SolverTerminalReason};
use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::SolverScope;
use crate::stats::SolverStats;
use crate::termination::Termination;

/* Result of a solve operation containing solution and telemetry.

This is the canonical return type for `Solver::solve()`. It provides
both the optimized solution and comprehensive statistics about the
solving process.
*/
#[derive(Debug)]
pub struct SolveResult<S: PlanningSolution> {
    // The best solution found during solving.
    pub solution: S,
    // The final working score when solving stopped.
    pub current_score: Option<S::Score>,
    // The canonical best score for the solve.
    pub best_score: S::Score,
    // Why solving stopped.
    pub terminal_reason: SolverTerminalReason,
    // Solver statistics including steps, moves evaluated, and acceptance rates.
    pub stats: SolverStats,
}

impl<S: PlanningSolution> SolveResult<S> {
    pub fn solution(&self) -> &S {
        &self.solution
    }

    pub fn into_solution(self) -> S {
        self.solution
    }

    pub fn current_score(&self) -> Option<&S::Score> {
        self.current_score.as_ref()
    }

    pub fn best_score(&self) -> &S::Score {
        &self.best_score
    }

    pub fn terminal_reason(&self) -> SolverTerminalReason {
        self.terminal_reason
    }

    pub fn stats(&self) -> &SolverStats {
        &self.stats
    }

    pub fn step_count(&self) -> u64 {
        self.stats.step_count
    }

    pub fn moves_evaluated(&self) -> u64 {
        self.stats.moves_evaluated
    }

    pub fn moves_accepted(&self) -> u64 {
        self.stats.moves_accepted
    }
}

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
/// * `ProgressCb` - Progress callback type (default `()`)
pub struct Solver<'t, P, T, S: PlanningSolution, D, ProgressCb = ()> {
    phases: P,
    termination: T,
    terminate: Option<&'t AtomicBool>,
    runtime: Option<SolverRuntime<S>>,
    config: Option<SolverConfig>,
    time_limit: Option<Duration>,
    // Callback invoked when the solver should publish progress.
    progress_callback: ProgressCb,
    _phantom: PhantomData<fn(S, D)>,
}

impl<P: Debug, T: Debug, S: PlanningSolution, D, ProgressCb> Debug
    for Solver<'_, P, T, S, D, ProgressCb>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("phases", &self.phases)
            .field("termination", &self.termination)
            .finish()
    }
}

impl<P, S, D> Solver<'static, P, NoTermination, S, D, ()>
where
    S: PlanningSolution,
{
    pub fn new(phases: P) -> Self {
        Solver {
            phases,
            termination: NoTermination,
            terminate: None,
            runtime: None,
            config: None,
            time_limit: None,
            progress_callback: (),
            _phantom: PhantomData,
        }
    }

    pub fn with_termination<T>(self, termination: T) -> Solver<'static, P, Option<T>, S, D, ()> {
        Solver {
            phases: self.phases,
            termination: Some(termination),
            terminate: self.terminate,
            runtime: self.runtime,
            config: self.config,
            time_limit: self.time_limit,
            progress_callback: self.progress_callback,
            _phantom: PhantomData,
        }
    }
}

impl<'t, P, T, S, D, ProgressCb> Solver<'t, P, T, S, D, ProgressCb>
where
    S: PlanningSolution,
{
    /// Sets the external termination flag.
    ///
    /// The solver will check this flag periodically and terminate early if set.
    pub fn with_terminate(self, terminate: &'t AtomicBool) -> Solver<'t, P, T, S, D, ProgressCb> {
        Solver {
            phases: self.phases,
            termination: self.termination,
            terminate: Some(terminate),
            runtime: self.runtime,
            config: self.config,
            time_limit: self.time_limit,
            progress_callback: self.progress_callback,
            _phantom: PhantomData,
        }
    }

    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    pub fn with_config(mut self, config: SolverConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Sets a callback to be invoked for exact solver progress and best-solution updates.
    ///
    /// Transitions the callback type parameter to the concrete closure type.
    pub fn with_progress_callback<F>(self, callback: F) -> Solver<'t, P, T, S, D, F> {
        Solver {
            phases: self.phases,
            termination: self.termination,
            terminate: self.terminate,
            runtime: self.runtime,
            config: self.config,
            time_limit: self.time_limit,
            progress_callback: callback,
            _phantom: PhantomData,
        }
    }

    pub fn config(&self) -> Option<&SolverConfig> {
        self.config.as_ref()
    }

    pub(crate) fn with_runtime(mut self, runtime: SolverRuntime<S>) -> Self {
        self.runtime = Some(runtime);
        self
    }
}

// Marker type indicating no termination.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTermination;

/// Marker trait for termination types that can be used in Solver.
pub trait MaybeTermination<
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S> = (),
>: Send
{
    // Checks if the solver should terminate.
    fn should_terminate(&self, solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool;

    /* Installs in-phase termination limits on the solver scope.

    This allows `Termination` conditions (step count, move count, etc.) to fire
    inside the phase step loop, not only between phases (T1 fix).

    The default implementation is a no-op. Override for terminations that
    express a concrete limit via a scope field.
    */
    fn install_inphase_limits(&self, _solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {}
}

impl<S, D, ProgressCb, T> MaybeTermination<S, D, ProgressCb> for Option<T>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    T: Termination<S, D, ProgressCb>,
{
    fn should_terminate(&self, solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool {
        match self {
            Some(t) => t.is_terminated(solver_scope),
            None => false,
        }
    }

    fn install_inphase_limits(&self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        if let Some(t) = self {
            t.install_inphase_limits(solver_scope);
        }
    }
}

impl<S, D, ProgressCb> MaybeTermination<S, D, ProgressCb> for NoTermination
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn should_terminate(&self, _solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool {
        false
    }

    // install_inphase_limits: no-op (default)
}

impl<S, D, ProgressCb> Termination<S, D, ProgressCb> for NoTermination
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn is_terminated(&self, _solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool {
        false
    }
}

macro_rules! impl_solver {
    ($($idx:tt: $P:ident),+) => {
        impl<'t, S, D, T, ProgressCb, $($P),+> Solver<'t, ($($P,)+), T, S, D, ProgressCb>
        where
            S: PlanningSolution,
            D: Director<S>,
            T: MaybeTermination<S, D, ProgressCb>,
            ProgressCb: ProgressCallback<S>,
            $($P: Phase<S, D, ProgressCb>,)+
        {
            /// Solves using the provided score director.
            ///
            /// Returns a `SolveResult` containing the best solution found
            /// and comprehensive solver statistics.
            pub fn solve(self, score_director: D) -> SolveResult<S> {
                let Solver {
                    mut phases,
                    termination,
                    terminate,
                    runtime,
                    config,
                    time_limit,
                    progress_callback,
                    ..
                } = self;

                let mut solver_scope = SolverScope::new_with_callback(
                    score_director,
                    progress_callback,
                    terminate,
                    runtime,
                );
                if let Some(environment_mode) = config.as_ref().map(|cfg| cfg.environment_mode) {
                    solver_scope = solver_scope.with_environment_mode(environment_mode);
                }
                if let Some(seed) = config.as_ref().and_then(|cfg| cfg.random_seed) {
                    solver_scope = solver_scope.with_seed(seed);
                }
                if let Some(limit) = time_limit {
                    solver_scope.set_time_limit(limit);
                }
                solver_scope.initialize_working_solution_as_best();
                solver_scope.report_best_solution();
                solver_scope.pause_if_requested();

                // Install in-phase termination limits so phases can check them
                // inside their step loops (T1: StepCountTermination, MoveCountTermination, etc.)
                termination.install_inphase_limits(&mut solver_scope);

                // Execute phases with termination checking
                $(
                    solver_scope.pause_if_requested();
                    if !check_termination(&termination, &mut solver_scope) {
                        tracing::debug!(
                            "Starting phase {} ({})",
                            $idx,
                            phases.$idx.phase_type_name()
                        );
                        phases.$idx.solve(&mut solver_scope);
                        solver_scope.pause_if_requested();
                        tracing::debug!(
                            "Finished phase {} ({}) with score {:?}",
                            $idx,
                            phases.$idx.phase_type_name(),
                            solver_scope.best_score()
                        );
                    }
                )+

                // Extract solution and stats before consuming scope
                let (solution, current_score, best_score, stats, terminal_reason) =
                    solver_scope.take_solution_and_stats();
                SolveResult {
                    solution,
                    current_score,
                    best_score,
                    terminal_reason,
                    stats,
                }
            }
        }
    };
}

fn check_termination<S, D, ProgressCb, T>(
    termination: &T,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) -> bool
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    T: MaybeTermination<S, D, ProgressCb>,
{
    if solver_scope.is_terminate_early() {
        solver_scope.mark_cancelled();
        return true;
    }
    if termination.should_terminate(solver_scope) {
        solver_scope.mark_terminated_by_config();
        true
    } else {
        false
    }
}

macro_rules! impl_solver_with_director {
    ($($idx:tt: $P:ident),+) => {
        impl<'t, S, T, ProgressCb, $($P),+> Solver<'t, ($($P,)+), T, S, (), ProgressCb>
        where
            S: PlanningSolution,
            T: Send,
            ProgressCb: Send + Sync,
        {
            /// Solves using a provided score director.
            pub fn solve_with_director<D>(self, director: D) -> SolveResult<S>
            where
                D: Director<S>,
                ProgressCb: ProgressCallback<S>,
                T: MaybeTermination<S, D, ProgressCb>,
                $($P: Phase<S, D, ProgressCb>,)+
            {
                let solver: Solver<'t, ($($P,)+), T, S, D, ProgressCb> = Solver {
                    phases: self.phases,
                    termination: self.termination,
                    terminate: self.terminate,
                    runtime: self.runtime,
                    config: self.config,
                    time_limit: self.time_limit,
                    progress_callback: self.progress_callback,
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
