//! One lowered local-search phase over retained compiled selector state.
//!
//! The phase owns only frozen selector state and acceptor/forager policy. The
//! enclosing runner retains the provider registry/reason arena and lends it
//! to the shared LocalSearch/VND loops at each cursor boundary.

use std::fmt::Debug;

use solverforge_config::{AcceptorConfig, ForagerConfig, ScoreTieBreak};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::{AcceptorBuilder, AnyAcceptor, AnyForager, ForagerBuilder};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::localsearch::vnd::solve_vnd_with_resources;
use crate::phase::localsearch::{
    solve_local_search_with_resources, AcceptedCountForager, BestScoreForager,
    DiversifiedLateAcceptanceAcceptor, FirstLastStepScoreImprovingForager, LateAcceptanceAcceptor,
    SimulatedAnnealingAcceptor,
};
use crate::runtime::compiler::{
    DefaultLocalSearchAcceptorPolicy, DefaultLocalSearchComponents, DefaultLocalSearchForagerPolicy,
};
use crate::scope::{ProgressCallback, SolverScope};

use super::super::local_search::{ProviderExecutionResources, RuntimeNeighborhoodState};

#[expect(
    clippy::large_enum_variant,
    reason = "runtime local search owns its per-solve state without heap dispatch"
)]
pub(super) enum RuntimeLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    AcceptorForager {
        phase_termination: Option<solverforge_config::TerminationConfig>,
        selector: RuntimeNeighborhoodState<S, V, DM, IDM>,
        components: DefaultLocalSearchComponents,
        acceptor: Option<AcceptorConfig>,
        forager: Option<ForagerConfig>,
        score_tie_break: ScoreTieBreak,
        random_seed: Option<u64>,
    },
    VariableNeighborhoodDescent {
        phase_termination: Option<solverforge_config::TerminationConfig>,
        neighborhoods: Vec<RuntimeNeighborhoodState<S, V, DM, IDM>>,
    },
}

impl<S, V, DM, IDM> RuntimeLocalSearch<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    pub(super) fn solve<D, ProgressCb>(
        &mut self,
        resources: &mut ProviderExecutionResources<S>,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        match self {
            Self::AcceptorForager {
                phase_termination,
                selector,
                components,
                acceptor,
                forager,
                score_tie_break,
                random_seed,
            } => {
                let (mut acceptor, mut forager) = build_acceptor_and_forager::<S>(
                    *components,
                    acceptor.as_ref(),
                    forager.as_ref(),
                    *score_tie_break,
                    *random_seed,
                );
                let mut source = selector.borrowed_source();
                solver_scope.with_phase_termination(phase_termination.as_ref(), |solver_scope| {
                    solve_local_search_with_resources(
                        &mut source,
                        resources,
                        &mut acceptor,
                        &mut forager,
                        None,
                        solver_scope,
                    );
                });
            }
            Self::VariableNeighborhoodDescent {
                phase_termination,
                neighborhoods,
            } => {
                let mut sources = neighborhoods
                    .iter_mut()
                    .map(RuntimeNeighborhoodState::borrowed_source)
                    .collect::<Vec<_>>();
                solver_scope.with_phase_termination(phase_termination.as_ref(), |solver_scope| {
                    solve_vnd_with_resources(&mut sources, resources, None, solver_scope);
                });
            }
        }
    }
}

pub(super) fn build_acceptor_and_forager<S>(
    components: DefaultLocalSearchComponents,
    configured_acceptor: Option<&AcceptorConfig>,
    configured_forager: Option<&ForagerConfig>,
    score_tie_break: ScoreTieBreak,
    random_seed: Option<u64>,
) -> (AnyAcceptor<S>, AnyForager<S>)
where
    S: PlanningSolution,
    S::Score: Score + ParseableScore,
{
    let random_ties = matches!(score_tie_break, ScoreTieBreak::Random);
    let acceptor = configured_acceptor.map_or_else(
        || build_default_acceptor(components.acceptor),
        |config| AcceptorBuilder::build_with_seed(config, random_seed),
    );
    let forager = configured_forager.map_or_else(
        || {
            if configured_acceptor
                .is_some_and(|acceptor| matches!(acceptor, AcceptorConfig::TabuSearch(_)))
            {
                AnyForager::BestScore(BestScoreForager::new(random_ties))
            } else {
                build_default_forager(components.forager, random_ties)
            }
        },
        |config| ForagerBuilder::build(Some(config), score_tie_break),
    );
    (acceptor, forager)
}

fn build_default_acceptor<S>(policy: DefaultLocalSearchAcceptorPolicy) -> AnyAcceptor<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    match policy {
        DefaultLocalSearchAcceptorPolicy::LateAcceptance { history_size } => {
            AnyAcceptor::LateAcceptance(LateAcceptanceAcceptor::new(history_size))
        }
        DefaultLocalSearchAcceptorPolicy::DiversifiedLateAcceptance { history_size } => {
            AnyAcceptor::DiversifiedLateAcceptance(
                DiversifiedLateAcceptanceAcceptor::with_default_tolerance(history_size),
            )
        }
        DefaultLocalSearchAcceptorPolicy::SimulatedAnnealing {
            decay_rate_bits,
            random_seed,
        } => match random_seed {
            Some(seed) => AnyAcceptor::SimulatedAnnealing(
                SimulatedAnnealingAcceptor::auto_calibrate_with_seed(
                    f64::from_bits(decay_rate_bits),
                    seed,
                ),
            ),
            None => AnyAcceptor::SimulatedAnnealing(SimulatedAnnealingAcceptor::default()),
        },
    }
}

fn build_default_forager<S>(
    policy: DefaultLocalSearchForagerPolicy,
    random_ties: bool,
) -> AnyForager<S>
where
    S: PlanningSolution,
    S::Score: Score,
{
    match policy {
        DefaultLocalSearchForagerPolicy::AcceptedCount { limit } => {
            AnyForager::AcceptedCount(AcceptedCountForager::new(limit, random_ties))
        }
        DefaultLocalSearchForagerPolicy::FirstLastStepScoreImproving {
            accepted_count_limit,
        } => {
            let forager = match accepted_count_limit {
                Some(limit) => FirstLastStepScoreImprovingForager::new(random_ties)
                    .with_accepted_count_limit(limit),
                None => FirstLastStepScoreImprovingForager::new(random_ties),
            };
            AnyForager::LastStepScoreImproving(forager)
        }
    }
}
