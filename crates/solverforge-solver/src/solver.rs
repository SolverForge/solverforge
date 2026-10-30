//! Solver implementation.

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
use crate::scope::WeightModeControl;
use crate::stats::{
    CandidateTraceExecutionPolicy, CandidateTraceHeader, CandidateTracePhasePlan,
    QualifiedCandidateTraceRunProvenance, SolverStats,
};
use crate::termination::Termination;

/// Result of a solve operation containing solution and telemetry.
///
/// This is what `SolveSession::finish` returns. It provides
/// both the optimized solution and comprehensive statistics about the
/// solving process.
#[derive(Debug)]
pub(crate) struct SolveResult<S: PlanningSolution> {
    /// The best solution found during solving.
    pub(crate) solution: S,
    /// The final working score when solving stopped.
    pub(crate) current_score: Option<S::Score>,
    /// The canonical best score for the solve.
    pub(crate) best_score: S::Score,
    /// Why solving stopped.
    pub(crate) terminal_reason: SolverTerminalReason,
    /// Solver statistics including steps, moves evaluated, and acceptance rates.
    pub(crate) stats: SolverStats,
}

/// The main solver that optimizes planning solutions.
///
/// Runs one phase (the compiled runtime runner in production) and keeps its
/// concrete type through the entire pipeline (zero-erasure architecture).
///
/// # Type Parameters
/// * `'t` - Lifetime of the termination flag reference
/// * `P` - The phase to execute
/// * `T` - Termination condition (use `Option<ConcreteTermination>`)
/// * `S` - Solution type
/// * `D` - Score director type
/// * `ProgressCb` - Progress callback type (default `()`)
pub(crate) struct Solver<'t, P, T, S: PlanningSolution, D, ProgressCb = ()> {
    phase: P,
    termination: T,
    terminate: Option<&'t AtomicBool>,
    runtime: Option<SolverRuntime<S>>,
    config: Option<SolverConfig>,
    /// Canonical resolved policy supplied by the configured runtime entrypoint.
    /// Direct generic `Solver` construction leaves this absent on purpose: an
    /// arbitrary Rust termination cannot be reconstructed honestly from the
    /// input config alone.
    candidate_trace_execution_policy: Option<CandidateTraceExecutionPolicy>,
    /// Explicit fail-closed benchmark attestation for qualified comparisons.
    candidate_trace_qualified_run_provenance: Option<QualifiedCandidateTraceRunProvenance>,
    time_limit: Option<Duration>,
    // Callback invoked when the solver should publish progress.
    progress_callback: ProgressCb,
    // How problem changes may switch the director's constraint weight mode.
    weight_mode_control: WeightModeControl,
    _phantom: PhantomData<fn(S, D)>,
}

impl<P: Debug, T: Debug, S: PlanningSolution, D, ProgressCb> Debug
    for Solver<'_, P, T, S, D, ProgressCb>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Solver")
            .field("phase", &self.phase)
            .field("termination", &self.termination)
            .finish()
    }
}

impl<P, S, D> Solver<'static, P, NoTermination, S, D, ()>
where
    S: PlanningSolution,
{
    pub(crate) fn new(phase: P) -> Self {
        Solver {
            phase,
            termination: NoTermination,
            terminate: None,
            runtime: None,
            config: None,
            candidate_trace_execution_policy: None,
            candidate_trace_qualified_run_provenance: None,
            time_limit: None,
            progress_callback: (),
            weight_mode_control: WeightModeControl::Fixed,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_termination<T>(
        self,
        termination: T,
    ) -> Solver<'static, P, Option<T>, S, D, ()> {
        Solver {
            phase: self.phase,
            termination: Some(termination),
            terminate: self.terminate,
            runtime: self.runtime,
            config: self.config,
            candidate_trace_execution_policy: self.candidate_trace_execution_policy,
            candidate_trace_qualified_run_provenance: self.candidate_trace_qualified_run_provenance,
            time_limit: self.time_limit,
            progress_callback: self.progress_callback,
            weight_mode_control: self.weight_mode_control,
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
    pub(crate) fn with_terminate(
        self,
        terminate: &'t AtomicBool,
    ) -> Solver<'t, P, T, S, D, ProgressCb> {
        Solver {
            phase: self.phase,
            termination: self.termination,
            terminate: Some(terminate),
            runtime: self.runtime,
            config: self.config,
            candidate_trace_execution_policy: self.candidate_trace_execution_policy,
            candidate_trace_qualified_run_provenance: self.candidate_trace_qualified_run_provenance,
            time_limit: self.time_limit,
            progress_callback: self.progress_callback,
            weight_mode_control: self.weight_mode_control,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    pub(crate) fn with_config(mut self, config: SolverConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Supplies the canonical policy resolved by a first-party entrypoint.
    ///
    /// This remains crate-private so external users cannot accidentally claim
    /// an arbitrary custom termination is equivalent to a built-in policy.
    pub(crate) fn with_candidate_trace_execution_policy(
        mut self,
        execution_policy: CandidateTraceExecutionPolicy,
    ) -> Self {
        self.candidate_trace_execution_policy = Some(execution_policy);
        self
    }

    /// Requests a qualified candidate trace with a complete, immutable
    /// model/instance/state/core-tree/build attestation. Construction of the
    /// value validates the attestation before execution; it is never inferred
    /// from environment, callbacks, or working state.
    pub(crate) fn with_qualified_candidate_trace_run_provenance(
        mut self,
        provenance: QualifiedCandidateTraceRunProvenance,
    ) -> Self {
        self.candidate_trace_qualified_run_provenance = Some(provenance);
        self
    }

    /// Sets a callback to be invoked for exact solver progress and best-solution updates.
    ///
    /// Transitions the callback type parameter to the concrete closure type.
    pub(crate) fn with_progress_callback<F>(self, callback: F) -> Solver<'t, P, T, S, D, F> {
        Solver {
            phase: self.phase,
            termination: self.termination,
            terminate: self.terminate,
            runtime: self.runtime,
            config: self.config,
            candidate_trace_execution_policy: self.candidate_trace_execution_policy,
            candidate_trace_qualified_run_provenance: self.candidate_trace_qualified_run_provenance,
            time_limit: self.time_limit,
            progress_callback: callback,
            weight_mode_control: self.weight_mode_control,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_runtime(mut self, runtime: SolverRuntime<S>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub(crate) fn with_weight_mode_control(mut self, control: WeightModeControl) -> Self {
        self.weight_mode_control = control;
        self
    }
}

/// Marker type indicating no termination.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct NoTermination;

/// Marker trait for termination types that can be used in Solver.
pub(crate) trait MaybeTermination<
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S> = (),
>: Send
{
    /// Checks if the solver should terminate.
    fn should_terminate(&mut self, solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool;

    /// Installs in-phase termination limits on the solver scope.
    ///
    /// This allows `Termination` conditions (step count, move count, etc.) to fire
    /// inside the phase step loop, not only between phases (T1 fix).
    ///
    /// The default implementation is a no-op. Override for terminations that
    /// express a concrete limit via a scope field.
    fn install_inphase_limits(&self, _solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {}
}

impl<S, D, ProgressCb, T> MaybeTermination<S, D, ProgressCb> for Option<T>
where
    S: PlanningSolution,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
    T: Termination<S, D, ProgressCb>,
{
    fn should_terminate(&mut self, solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool {
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
    fn should_terminate(&mut self, _solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool {
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
    fn is_terminated(&mut self, _solver_scope: &SolverScope<'_, S, D, ProgressCb>) -> bool {
        false
    }
}

mod session;

pub(crate) use session::SolveSession;

#[cfg(test)]
#[path = "solver_tests.rs"]
mod tests;
