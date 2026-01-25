//! Zero-erasure builder for SolverFactory.

use std::marker::PhantomData;
use std::time::Duration;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::api::constraint_set::ConstraintSet;

use crate::phase::Phase;
use crate::solver::NoTermination;
use crate::termination::{OrTermination, StepCountTermination, Termination, TimeTermination};

use super::SolverFactory;

/// Builder for SolverFactory with zero type erasure.
///
/// Accumulates configuration and produces a fully typed SolverFactory.
/// Type bounds are only checked at `build()` time.
pub struct SolverFactoryBuilder<S, C, Calc, P, T>
where
    S: PlanningSolution,
{
    score_calculator: Calc,
    phases: P,
    termination: T,
    _marker: PhantomData<fn(S, C)>,
}

impl<S, C, Calc> SolverFactoryBuilder<S, C, Calc, (), NoTermination>
where
    S: PlanningSolution,
    Calc: Fn(&S) -> S::Score + Send + Sync,
{
    /// Creates a new builder with a score calculator.
    pub fn new(score_calculator: Calc) -> Self {
        Self {
            score_calculator,
            phases: (),
            termination: NoTermination,
            _marker: PhantomData,
        }
    }
}

impl<S, C, Calc, P, T> SolverFactoryBuilder<S, C, Calc, P, T>
where
    S: PlanningSolution,
{
    /// Adds a phase, returning a new builder with updated phase tuple.
    pub fn with_phase<P2>(self, phase: P2) -> SolverFactoryBuilder<S, C, Calc, (P, P2), T> {
        SolverFactoryBuilder {
            score_calculator: self.score_calculator,
            phases: (self.phases, phase),
            termination: self.termination,
            _marker: PhantomData,
        }
    }

    /// Applies configuration from a SolverConfig.
    ///
    /// Currently applies termination settings from the config.
    pub fn with_config(
        self,
        config: SolverConfig,
    ) -> SolverFactoryBuilder<S, C, Calc, P, TimeTermination> {
        let term = config.termination.unwrap_or_default();
        let duration = term.time_limit().unwrap_or(Duration::from_secs(30));
        SolverFactoryBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: TimeTermination::new(duration),
            _marker: PhantomData,
        }
    }

    /// Sets time limit termination.
    pub fn with_time_limit(
        self,
        duration: Duration,
    ) -> SolverFactoryBuilder<S, C, Calc, P, TimeTermination> {
        SolverFactoryBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: TimeTermination::new(duration),
            _marker: PhantomData,
        }
    }

    /// Sets step limit termination.
    pub fn with_step_limit(
        self,
        steps: u64,
    ) -> SolverFactoryBuilder<S, C, Calc, P, StepCountTermination> {
        SolverFactoryBuilder {
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
    ) -> SolverFactoryBuilder<S, C, Calc, P, OrTermination<(T, TimeTermination), S, C>>
    where
        S::Score: Score,
        C: ConstraintSet<S, S::Score>,
    {
        SolverFactoryBuilder {
            score_calculator: self.score_calculator,
            phases: self.phases,
            termination: OrTermination::new((self.termination, TimeTermination::new(duration))),
            _marker: PhantomData,
        }
    }
}

impl<S, C, Calc, P, T> SolverFactoryBuilder<S, C, Calc, P, T>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
    Calc: Fn(&S) -> S::Score + Send + Sync,
    P: Phase<S, C>,
    T: Termination<S, C>,
{
    /// Builds the SolverFactory.
    ///
    /// Returns `Ok(SolverFactory)` on success, or `Err` if configuration is invalid.
    /// Currently always succeeds as validation happens at compile time via type bounds.
    pub fn build(self) -> Result<SolverFactory<S, C, Calc, P, T>, SolverBuildError> {
        Ok(SolverFactory::new(
            self.score_calculator,
            self.phases,
            self.termination,
        ))
    }
}

/// Error type for SolverFactory building.
#[derive(Debug, Clone)]
pub enum SolverBuildError {
    /// Configuration is invalid.
    InvalidConfig(String),
}

impl std::fmt::Display for SolverBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverBuildError::InvalidConfig(msg) => {
                write!(f, "Invalid solver configuration: {}", msg)
            }
        }
    }
}

impl std::error::Error for SolverBuildError {}
