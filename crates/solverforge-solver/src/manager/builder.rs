//! Builder for SolverManager configuration.

use solverforge_core::domain::PlanningSolution;
use solverforge_core::SolverForgeError;
use solverforge_scoring::ScoreDirector;

use crate::termination::{
    DiminishedReturnsTermination, OrCompositeTermination, StepCountTermination, Termination,
    TimeTermination,
};

use super::{SolverManager, SolverPhaseFactory};

/// Builder for creating a [`SolverManager`].
pub struct SolverManagerBuilder<S: PlanningSolution, D: ScoreDirector<S>> {
    phase_factories: Vec<Box<dyn SolverPhaseFactory<S, D>>>,
    config: Option<solverforge_config::SolverConfig>,
}

impl<S: PlanningSolution + 'static, D: ScoreDirector<S> + 'static> SolverManagerBuilder<S, D> {
    pub fn new() -> Self {
        Self {
            phase_factories: Vec::new(),
            config: None,
        }
    }

    pub fn with_config(mut self, config: solverforge_config::SolverConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_phase_factory<F: SolverPhaseFactory<S, D> + 'static>(
        mut self,
        factory: F,
    ) -> Self {
        self.phase_factories.push(Box::new(factory));
        self
    }

    pub fn build(self) -> Result<SolverManager<S, D>, SolverForgeError> {
        let termination_factory = self.build_termination_factory();
        Ok(SolverManager::new(self.phase_factories, termination_factory))
    }

    fn build_termination_factory(
        &self,
    ) -> Option<Box<dyn Fn() -> Box<dyn Termination<S, D>> + Send + Sync>> {
        let config = self.config.clone()?;
        let termination = config.termination?;

        let time_limit = termination.time_limit();
        let step_limit = termination.step_count_limit;
        let unimproved_time = termination.unimproved_time_limit();

        if time_limit.is_none() && step_limit.is_none() && unimproved_time.is_none() {
            return None;
        }

        Some(Box::new(move || {
            let mut terminations: Vec<Box<dyn Termination<S, D>>> = Vec::new();

            if let Some(duration) = time_limit {
                terminations.push(Box::new(TimeTermination::new(duration)));
            }

            if let Some(steps) = step_limit {
                terminations.push(Box::new(StepCountTermination::new(steps)));
            }

            if let Some(duration) = unimproved_time {
                terminations.push(Box::new(DiminishedReturnsTermination::<S>::new(
                    duration, 0.001,
                )));
            }

            match terminations.len() {
                0 => unreachable!(),
                1 => terminations.remove(0),
                _ => Box::new(OrCompositeTermination::new(terminations)),
            }
        }))
    }
}

impl<S: PlanningSolution + 'static, D: ScoreDirector<S> + 'static> Default
    for SolverManagerBuilder<S, D>
{
    fn default() -> Self {
        Self::new()
    }
}
