//! Zero-erasure builder for SolverManager.

use std::marker::PhantomData;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::termination::{OrTermination, StepCountTermination, Termination, TimeTermination};
use crate::solver::NoTermination;

use super::{PhaseFactory, SolverManager};

/// Builder for SolverManager with zero type erasure.
///
/// Accumulates configuration and produces a fully typed SolverManager.
/// Type bounds are only checked at `build()` time.
pub struct SolverManagerBuilder<S, D, C, P, T>
where
    S: PlanningSolution,
{
    score_calculator: C,
    phases: P,
    termination: T,
    _marker: PhantomData<fn(S, D)>,
}

impl<S, D, C> SolverManagerBuilder<S, D, C, (), NoTermination>
where
    S: PlanningSolution,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a new builder with a score calculator.
    pub fn new(score_calculator: C) -> Self {
        Self {
            score_calculator,
            phases: (),
            termination: NoTermination,
            _marker: PhantomData,
        }
    }
}

impl<S, D, C, P, T> SolverManagerBuilder<S, D, C, P, T>
where
    S: PlanningSolution,
{
    /// Adds a phase, returning a new builder with updated phase tuple.
    pub fn with_phase<P2>(self, phase: P2) -> SolverManagerBuilder<S, D, C, (P, P2), T> {
        SolverManagerBuilder {
            score_calculator: self.score_calculator,
            phases: (self.phases, phase),
            termination: self.termination,
            _marker: PhantomData,
        }
    }

    /// Adds a phase from a factory, returning a new builder with updated phase tuple.
    ///
    /// The factory's `create()` method is called to produce the phase.
    pub fn with_phase_factory<F>(self, factory: F) -> SolverManagerBuilder<S, D, C, (P, F::Phase), T>
    where
        D: ScoreDirector<S>,
        F: PhaseFactory<S, D>,
    {
        let phase = factory.create();
        SolverManagerBuilder {
            score_calculator: self.score_calculator,
            phases: (self.phases, phase),
            termination: self.termination,
            _marker: PhantomData,
        }
    }

    /// Applies configuration from a SolverConfig.
    ///
    /// Currently applies termination settings from the config.
    pub fn with_config(self, config: SolverConfig) -> SolverManagerBuilder<S, D, C, P, TimeTermination> {
        let term = config.termination.unwrap_or_default();
        let duration = term.time_limit().unwrap_or(Duration::from_secs(30));
        SolverManagerBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: TimeTermination::new(duration),
            _marker: PhantomData,
        }
    }

    /// Sets time limit termination.
    pub fn with_time_limit(self, duration: Duration) -> SolverManagerBuilder<S, D, C, P, TimeTermination> {
        SolverManagerBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: TimeTermination::new(duration),
            _marker: PhantomData,
        }
    }

    /// Sets step limit termination.
    pub fn with_step_limit(self, steps: u64) -> SolverManagerBuilder<S, D, C, P, StepCountTermination> {
        SolverManagerBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: StepCountTermination::new(steps),
            _marker: PhantomData,
        }
    }

    /// Combines current termination with time limit.
    #[allow(clippy::type_complexity)]
    pub fn with_time_limit_or(
        self,
        duration: Duration,
    ) -> SolverManagerBuilder<S, D, C, P, OrTermination<(T, TimeTermination), S, D>>
    where
        D: ScoreDirector<S>,
    {
        SolverManagerBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: OrTermination::new((self.termination, TimeTermination::new(duration))),
            _marker: PhantomData,
        }
    }
}

impl<S, D, C, P, T> SolverManagerBuilder<S, D, C, P, T>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
    P: Phase<S, D>,
    T: Termination<S, D>,
{
    /// Builds the SolverManager.
    ///
    /// Returns `Ok(SolverManager)` on success, or `Err` if configuration is invalid.
    /// Currently always succeeds as validation happens at compile time via type bounds.
    pub fn build(self) -> Result<SolverManager<S, D, C, P, T>, SolverBuildError> {
        Ok(SolverManager::new(self.score_calculator, self.phases, self.termination))
    }
}

/// Error type for SolverManager building.
#[derive(Debug, Clone)]
pub enum SolverBuildError {
    /// Configuration is invalid.
    InvalidConfig(String),
}

impl std::fmt::Display for SolverBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverBuildError::InvalidConfig(msg) => write!(f, "Invalid solver configuration: {}", msg),
        }
    }
}

impl std::error::Error for SolverBuildError {}
