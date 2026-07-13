//! Lowering from one frozen graph phase into retained executable runner state.
//!
//! This is deliberately structural: it does not bind a list source, invoke a
//! provider, or inspect a working solution. Reached execution owns those
//! operations in the runner and shared kernels.

use std::fmt::Debug;

use solverforge_config::SolverConfig;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};

use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::runtime::compiler::{
    CompiledAcceptorForagerSelector, CompiledLocalSearch, CompiledRuntimeExtension,
    CompiledRuntimePhase, DefaultLocalSearchEligibility, DefaultRuntimeBindings,
};
use crate::runtime_build_error::{RuntimeBuildError, RuntimeBuildResult};
use crate::stats::CandidateTracePhasePlan;

use super::super::local_search::{lower_default_selector_union, lower_selector};
use super::super::trace::{extension_plan, local_search_plan, prepared_construction_plan};
use super::super::PreparedRuntimePhase;
use super::local_search::RuntimeLocalSearch;
use super::{RunnerPhase, RunnerPhaseOutcome};

pub(super) fn lower_runner_phase<S, V, DM, IDM, Extension>(
    phase_index: usize,
    prepared: &PreparedRuntimePhase<S, V, DM, IDM, Extension>,
    declared: &CompiledRuntimePhase<S, V, DM, IDM>,
    solver_config: &SolverConfig,
    defaults: &DefaultRuntimeBindings<S, V, DM, IDM>,
) -> RuntimeBuildResult<RunnerPhase<S, V, DM, IDM>>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    match (prepared, declared) {
        (
            PreparedRuntimePhase::Construction(construction),
            CompiledRuntimePhase::Construction(_),
        ) => Ok(RunnerPhase::Construction {
            phase_index,
            declaration: prepared_construction_plan(construction),
            outcome: RunnerPhaseOutcome::Pending,
        }),
        (PreparedRuntimePhase::LocalSearch(local_search), CompiledRuntimePhase::LocalSearch(_)) => {
            let (local_search, declaration) =
                lower_local_search(local_search, solver_config, defaults, phase_index)?;
            Ok(RunnerPhase::LocalSearch {
                phase_index,
                declaration,
                local_search,
                outcome: RunnerPhaseOutcome::Pending,
            })
        }
        (PreparedRuntimePhase::Extension(_), CompiledRuntimePhase::Extension(extension)) => {
            Ok(RunnerPhase::Extension {
                phase_index,
                declaration: extension_plan(extension_trace_kind(extension)),
                outcome: RunnerPhaseOutcome::Pending,
            })
        }
        (PreparedRuntimePhase::DefaultRuntime(_), CompiledRuntimePhase::DefaultRuntime) => {
            let (local_search, local_search_declaration) = match (
                defaults.local_search_plan.as_ref(),
                defaults.local_search_policy.eligibility::<S>(solver_config),
            ) {
                (Some(plan), DefaultLocalSearchEligibility::Eligible) => {
                    let selector = lower_default_selector_union(
                        solver_config,
                        plan.selection_order,
                        &defaults.local_search_nodes,
                    )
                    .map_err(|error| lowering_error(phase_index, error))?;
                    (
                        Some(RuntimeLocalSearch::AcceptorForager {
                            phase_termination: None,
                            selector,
                            components: plan.components,
                            acceptor: None,
                            forager: None,
                            score_tie_break: solverforge_config::ScoreTieBreak::Random,
                            random_seed: solver_config.random_seed,
                        }),
                        Some(plan.candidate_trace_plan()),
                    )
                }
                (None, _)
                | (Some(_), DefaultLocalSearchEligibility::IneligibleWithoutEffectiveTermination) => {
                    (None, None)
                }
            };
            Ok(RunnerPhase::DefaultRuntime {
                phase_index,
                local_search,
                local_search_declaration,
                construction_execution: None,
                outcome: RunnerPhaseOutcome::Pending,
            })
        }
        _ => Err(RuntimeBuildError::Preparation {
            phase_index,
            message: "prepared runtime phase no longer matches its frozen declaration".to_string(),
        }),
    }
}

fn lower_local_search<S, V, DM, IDM>(
    local_search: &CompiledLocalSearch<S, V, DM, IDM>,
    solver_config: &SolverConfig,
    defaults: &DefaultRuntimeBindings<S, V, DM, IDM>,
    phase_index: usize,
) -> RuntimeBuildResult<(RuntimeLocalSearch<S, V, DM, IDM>, CandidateTracePhasePlan)>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + ParseableScore,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    match local_search {
        CompiledLocalSearch::AcceptorForager { config, selector } => {
            let (selector, omitted_plan) = match selector {
                CompiledAcceptorForagerSelector::Explicit(selector) => (
                    lower_selector(solver_config, selector)
                        .map_err(|error| lowering_error(phase_index, error))?,
                    None,
                ),
                CompiledAcceptorForagerSelector::OmittedDefault => {
                    let plan = defaults.local_search_plan.as_ref().ok_or_else(|| {
                        RuntimeBuildError::Preparation {
                            phase_index,
                            message:
                                "omitted local-search selector retained no frozen default plan"
                                    .to_string(),
                        }
                    })?;
                    (
                        lower_default_selector_union(
                            solver_config,
                            plan.selection_order,
                            &defaults.local_search_nodes,
                        )
                        .map_err(|error| lowering_error(phase_index, error))?,
                        Some(plan),
                    )
                }
            };
            let omitted_selector_children =
                omitted_plan.map_or_else(Vec::new, |plan| plan.candidate_trace_selector_children());
            let declaration = local_search_plan(
                local_search,
                defaults.local_search_components,
                omitted_selector_children,
            );
            Ok((
                RuntimeLocalSearch::AcceptorForager {
                    phase_termination: config.termination.clone(),
                    selector,
                    components: defaults.local_search_components,
                    acceptor: config.acceptor.clone(),
                    forager: config.forager.clone(),
                    score_tie_break: config.score_tie_break,
                    random_seed: solver_config.random_seed,
                },
                declaration,
            ))
        }
        CompiledLocalSearch::VariableNeighborhoodDescent {
            config,
            neighborhoods,
        } => {
            let neighborhoods = neighborhoods
                .iter()
                .map(|neighborhood| {
                    lower_selector(solver_config, neighborhood)
                        .map_err(|error| lowering_error(phase_index, error))
                })
                .collect::<RuntimeBuildResult<Vec<_>>>()?;
            let declaration =
                local_search_plan(local_search, defaults.local_search_components, Vec::new());
            Ok((
                RuntimeLocalSearch::VariableNeighborhoodDescent {
                    phase_termination: config.termination.clone(),
                    neighborhoods,
                },
                declaration,
            ))
        }
    }
}

fn extension_trace_kind(extension: &CompiledRuntimeExtension) -> &'static str {
    match extension {
        CompiledRuntimeExtension::Custom { .. } => "custom",
        CompiledRuntimeExtension::Partitioned { .. } => "partitioned_search",
    }
}

fn lowering_error(
    phase_index: usize,
    error: super::super::local_search::RuntimeLocalSearchLoweringError,
) -> RuntimeBuildError {
    RuntimeBuildError::Preparation {
        phase_index,
        message: error.message().to_string(),
    }
}
