//! Builder for SolverManager configuration.
//!
//! Note: For zero-erasure architecture, use `SolverManager::new()` directly
//! with concrete phase and termination types.

use std::marker::PhantomData;
use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::solver::Solver;
use crate::termination::{StepCountTermination, Termination, TimeTermination};

/// Builder for creating solvers with configuration.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `P` - The phase type
pub struct SolverBuilder<S, D, P>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    phase: P,
    time_limit: Option<Duration>,
    step_limit: Option<u64>,
    _marker: PhantomData<(S, D)>,
}

impl<S, D, P> SolverBuilder<S, D, P>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    P: Phase<S, D>,
{
    /// Creates a new solver builder with the given phase.
    pub fn new(phase: P) -> Self {
        Self {
            phase,
            time_limit: None,
            step_limit: None,
            _marker: PhantomData,
        }
    }

    /// Sets a time limit for the solver.
    pub fn with_time_limit(mut self, duration: Duration) -> Self {
        self.time_limit = Some(duration);
        self
    }

    /// Sets a step count limit for the solver.
    pub fn with_step_limit(mut self, steps: u64) -> Self {
        self.step_limit = Some(steps);
        self
    }

    /// Builds a solver with time termination.
    pub fn build_with_time(self) -> Solver<S, D, P, TimeTermination>
    where
        TimeTermination: Termination<S, D>,
    {
        let termination = self
            .time_limit
            .map(TimeTermination::new);
        Solver::new(self.phase, termination)
    }

    /// Builds a solver with step count termination.
    pub fn build_with_steps(self) -> Solver<S, D, P, StepCountTermination>
    where
        StepCountTermination: Termination<S, D>,
    {
        let termination = self
            .step_limit
            .map(StepCountTermination::new);
        Solver::new(self.phase, termination)
    }

    /// Builds a solver without termination.
    pub fn build(self) -> Solver<S, D, P, ()> {
        Solver::with_phase(self.phase)
    }
}
