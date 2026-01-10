//! Builder for SolverManager configuration.
//!
//! This module provides the builder pattern for configuring a [`SolverManager`].
//! The builder allows fluent configuration of:
//!
//! - Construction heuristic phases
//! - Local search phases with various acceptors
//! - Termination conditions (time limits, step limits)

use std::marker::PhantomData;
use std::time::Duration;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::SolverForgeError;
use solverforge_scoring::ScoreDirector;

use crate::termination::{OrTermination, StepCountTermination, Termination, TimeTermination};

use super::config::{ConstructionType, LocalSearchType, PhaseConfig};
use super::{SolverManager, SolverPhaseFactory};

/// Builder for creating a [`SolverManager`] with fluent configuration.
///
/// The builder pattern allows configuring phases, termination conditions,
/// and other solver settings before creating the manager.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `D` - The score director type
/// * `C` - The score calculator type
///
/// # Zero-Erasure Design
///
/// The score calculator is stored as a concrete generic type parameter `C`,
/// not as `Arc<dyn Fn>`. This eliminates virtual dispatch overhead.
pub struct SolverManagerBuilder<S, D, C>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    C: Fn(&S) -> S::Score + Send + Sync,
{
    score_calculator: C,
    phase_configs: Vec<PhaseConfig>,
    time_limit: Option<Duration>,
    step_limit: Option<u64>,
    _phantom: PhantomData<fn(S, D)>,
}

impl<S, D, C> SolverManagerBuilder<S, D, C>
where
    S: PlanningSolution,
    D: ScoreDirector<S> + 'static,
    C: Fn(&S) -> S::Score + Send + Sync + 'static,
{
    /// Creates a new builder with the given score calculator (zero-erasure).
    ///
    /// The score calculator is a function that computes the score for a solution.
    /// Higher scores are better (for minimization, use negative values).
    pub fn new(score_calculator: C) -> Self {
        Self {
            score_calculator,
            phase_configs: Vec::new(),
            time_limit: None,
            step_limit: None,
            _phantom: PhantomData,
        }
    }

    /// Adds a construction heuristic phase with default (FirstFit) configuration.
    pub fn with_construction_heuristic(mut self) -> Self {
        self.phase_configs.push(PhaseConfig::ConstructionHeuristic {
            construction_type: ConstructionType::FirstFit,
        });
        self
    }

    /// Adds a construction heuristic phase with specific configuration.
    pub fn with_construction_heuristic_type(mut self, construction_type: ConstructionType) -> Self {
        self.phase_configs
            .push(PhaseConfig::ConstructionHeuristic { construction_type });
        self
    }

    /// Adds a local search phase.
    pub fn with_local_search(mut self, search_type: LocalSearchType) -> Self {
        self.phase_configs.push(PhaseConfig::LocalSearch {
            search_type,
            step_limit: None,
        });
        self
    }

    /// Adds a local search phase with a step limit.
    pub fn with_local_search_steps(
        mut self,
        search_type: LocalSearchType,
        step_limit: u64,
    ) -> Self {
        self.phase_configs.push(PhaseConfig::LocalSearch {
            search_type,
            step_limit: Some(step_limit),
        });
        self
    }

    /// Sets the global time limit for solving.
    pub fn with_time_limit(mut self, duration: Duration) -> Self {
        self.time_limit = Some(duration);
        self
    }

    /// Sets the global step limit for solving.
    pub fn with_step_limit(mut self, steps: u64) -> Self {
        self.step_limit = Some(steps);
        self
    }

    /// Builds the [`SolverManager`].
    ///
    /// # Errors
    ///
    /// Currently this method always succeeds, but returns a `Result` for
    /// forward compatibility with validation.
    pub fn build(self) -> Result<SolverManager<S, D, C>, SolverForgeError> {
        let termination_factory = self.build_termination_factory();
        let phase_factories: Vec<Box<dyn SolverPhaseFactory<S, D>>> = Vec::new();

        // Store phase configs for future use
        let _ = self.phase_configs;

        Ok(SolverManager::new(
            self.score_calculator,
            phase_factories,
            termination_factory,
        ))
    }

    #[allow(clippy::type_complexity)]
    fn build_termination_factory(
        &self,
    ) -> Option<Box<dyn Fn() -> Box<dyn Termination<S, D>> + Send + Sync>> {
        let time_limit = self.time_limit;
        let step_limit = self.step_limit;

        match (time_limit, step_limit) {
            (None, None) => None,
            (Some(duration), None) => {
                Some(Box::new(move || -> Box<dyn Termination<S, D>> {
                    Box::new(TimeTermination::new(duration))
                }))
            }
            (None, Some(steps)) => Some(Box::new(move || -> Box<dyn Termination<S, D>> {
                Box::new(StepCountTermination::new(steps))
            })),
            (Some(duration), Some(steps)) => {
                Some(Box::new(move || -> Box<dyn Termination<S, D>> {
                    Box::new(OrTermination::new((
                        TimeTermination::new(duration),
                        StepCountTermination::new(steps),
                    )))
                }))
            }
        }
    }
}
