//! Config bridge module for wiring config types to implementations.
//!
//! Provides conversion functions from `solverforge_config` types to
//! monomorphic impl enums using zero-erasure architecture.

use solverforge_config::{
    AcceptorConfig, ConstructionHeuristicConfig, ForagerConfig, LocalSearchConfig, SolverConfig,
    TerminationConfig,
};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::ConstraintSet;

use crate::heuristic::{Move, MoveSelector};
use crate::phase::construction::{
    ConstructionForagerImpl, ConstructionHeuristicPhase, EntityPlacer,
};
use crate::phase::localsearch::{AcceptorImpl, LocalSearchForagerImpl, LocalSearchPhase};
use crate::scope::SolverScope;
use crate::termination::{
    ScoreCalculationCountTermination, StepCountTermination, Termination, TimeTermination,
    UnimprovedStepCountTermination, UnimprovedTimeTermination,
};

/// Creates an acceptor from config.
pub fn acceptor_from_config<S>(config: &AcceptorConfig) -> AcceptorImpl<S>
where
    S: PlanningSolution,
{
    AcceptorImpl::from_config(config)
}

/// Creates a local search forager from config.
pub fn local_search_forager_from_config<S, M>(
    config: &ForagerConfig,
) -> LocalSearchForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    LocalSearchForagerImpl::from_config(config)
}

/// Creates a construction forager from config.
pub fn construction_forager_from_config<S, M>(
    config: &ConstructionHeuristicConfig,
) -> ConstructionForagerImpl<S, M>
where
    S: PlanningSolution,
    M: Move<S>,
{
    ConstructionForagerImpl::from_config(config.construction_heuristic_type)
}

/// Creates a construction heuristic phase from config.
pub fn construction_phase_from_config<S, M, EP>(
    config: &ConstructionHeuristicConfig,
    entity_placer: EP,
) -> ConstructionHeuristicPhase<S, M, EP, ConstructionForagerImpl<S, M>>
where
    S: PlanningSolution + solverforge_scoring::ShadowVariableSupport,
    M: Move<S>,
    EP: EntityPlacer<S, M>,
{
    let forager = construction_forager_from_config::<S, M>(config);
    ConstructionHeuristicPhase::new(entity_placer, forager)
}

/// Creates a local search phase from config.
pub fn local_search_phase_from_config<S, M, MS>(
    config: &LocalSearchConfig,
    move_selector: MS,
) -> LocalSearchPhase<S, M, MS, AcceptorImpl<S>, LocalSearchForagerImpl<S, M>>
where
    S: PlanningSolution,
    M: Move<S> + Send + Sync + 'static,
    MS: MoveSelector<S, M> + Clone + Send + Sync + 'static,
{
    let acceptor = config
        .acceptor
        .as_ref()
        .map(|c| AcceptorImpl::from_config(c))
        .unwrap_or_else(AcceptorImpl::late_acceptance);

    let forager = config
        .forager
        .as_ref()
        .map(|c| LocalSearchForagerImpl::from_config(c))
        .unwrap_or_else(|| LocalSearchForagerImpl::accepted_count(1000));

    LocalSearchPhase::new(move_selector, acceptor, forager, None)
}

/// Termination enum for config-driven termination.
pub enum TerminationImpl<S: PlanningSolution> {
    Time(TimeTermination),
    StepCount(StepCountTermination),
    ScoreCalculationCount(ScoreCalculationCountTermination<S>),
    UnimprovedStepCount(UnimprovedStepCountTermination<S>),
    UnimprovedTime(UnimprovedTimeTermination<S>),
}

impl<S: PlanningSolution> std::fmt::Debug for TerminationImpl<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Time(_) => write!(f, "TimeTermination"),
            Self::StepCount(_) => write!(f, "StepCountTermination"),
            Self::ScoreCalculationCount(_) => write!(f, "ScoreCalculationCountTermination"),
            Self::UnimprovedStepCount(_) => write!(f, "UnimprovedStepCountTermination"),
            Self::UnimprovedTime(_) => write!(f, "UnimprovedTimeTermination"),
        }
    }
}

unsafe impl<S: PlanningSolution> Send for TerminationImpl<S> {}

impl<S, C> Termination<S, C> for TerminationImpl<S>
where
    S: PlanningSolution,
    S::Score: Score,
    C: ConstraintSet<S, S::Score>,
{
    fn is_terminated(&self, solver_scope: &SolverScope<S, C>) -> bool {
        match self {
            Self::Time(t) => t.is_terminated(solver_scope),
            Self::StepCount(t) => t.is_terminated(solver_scope),
            Self::ScoreCalculationCount(t) => t.is_terminated(solver_scope),
            Self::UnimprovedStepCount(t) => t.is_terminated(solver_scope),
            Self::UnimprovedTime(t) => t.is_terminated(solver_scope),
        }
    }
}

/// Creates a termination from config.
pub fn termination_from_config<S>(config: &TerminationConfig) -> TerminationImpl<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    if let Some(secs) = config.seconds_spent_limit {
        return TerminationImpl::Time(TimeTermination::new(std::time::Duration::from_secs(secs)));
    }
    if let Some(mins) = config.minutes_spent_limit {
        return TerminationImpl::Time(TimeTermination::new(std::time::Duration::from_secs(
            mins * 60,
        )));
    }
    if let Some(count) = config.step_count_limit {
        return TerminationImpl::StepCount(StepCountTermination::new(count));
    }
    if let Some(count) = config.unimproved_step_count_limit {
        return TerminationImpl::UnimprovedStepCount(UnimprovedStepCountTermination::new(count));
    }
    if let Some(secs) = config.unimproved_seconds_spent_limit {
        return TerminationImpl::UnimprovedTime(UnimprovedTimeTermination::new(
            std::time::Duration::from_secs(secs),
        ));
    }

    TerminationImpl::Time(TimeTermination::new(std::time::Duration::from_secs(30)))
}

/// Creates a solver termination from config.
pub fn solver_termination_from_config<S>(config: &SolverConfig) -> Option<TerminationImpl<S>>
where
    S: PlanningSolution,
    S::Score: Score,
{
    config
        .termination
        .as_ref()
        .map(|tc| termination_from_config(tc))
}
