//! The one retained phase runner for a compiled runtime graph.
//!
//! It consumes only prepared graph nodes and lowered frozen selectors. In
//! particular, it never re-enters public selector builders, reconstructs a
//! construction model, or owns a second provider registry.

mod failure;
mod local_search;
mod lowering;

use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::{CustomSearchPhase, RuntimeExtensionRegistry};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::Phase;
use crate::runtime::finalize_noop_construction;
use crate::runtime_build_error::RuntimeBuildResult;
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::CandidateTracePhasePlan;

use super::completion::{publish_if_mandatory_complete, require_mandatory_completion};
use super::local_search::ProviderExecutionResources;
use super::trace::{default_construction_plan, phase_with_outcome};
use super::{
    CompiledRuntimeExecutor, ConstructionExecution, DefaultRuntimeConstructionExecution,
    PreparedRuntimeExecution, PreparedRuntimePhase,
};
use crate::runtime::compiler::DefaultRuntimeBindings;
use failure::{map_preparation_error, panic_execution_error, panic_runtime_execution_error};
use local_search::RuntimeLocalSearch;
use lowering::lower_runner_phase;

pub(crate) use failure::take_runtime_execution_failure;

/// One retained per-solve executor. A public configured entrypoint creates it
/// after graph compilation/preparation and hands it to the ordinary solver as
/// a single `Phase`; pause and resume happen inside that same object.
pub(crate) struct CompiledRuntimePhaseRunner<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    execution: PreparedRuntimeExecution<S, V, DM, IDM, Extension>,
    prepared_phases: Vec<PreparedRuntimePhase<S, V, DM, IDM, Extension>>,
    phases: Vec<RunnerPhase<S, V, DM, IDM>>,
    provider_resources: ProviderExecutionResources<S>,
    mandatory_bindings: DefaultRuntimeBindings<S, V, DM, IDM>,
    mandatory_completion_published: bool,
    terminal_notified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunnerPhaseOutcome {
    Pending,
    Executed,
    SkippedNoWork,
    SkippedTerminated,
}

impl RunnerPhaseOutcome {
    fn trace_label(self) -> &'static str {
        match self {
            Self::Pending => "not_reached_solver_termination",
            Self::Executed => "executed",
            Self::SkippedNoWork => "skipped_no_work",
            Self::SkippedTerminated => "skipped_terminated",
        }
    }

    fn from_construction(execution: ConstructionExecution) -> Self {
        match execution.outcome {
            super::ResolvedConstructionExecutionOutcome::Executed => Self::Executed,
            super::ResolvedConstructionExecutionOutcome::SkippedNoWork => Self::SkippedNoWork,
            super::ResolvedConstructionExecutionOutcome::SkippedTerminated => {
                Self::SkippedTerminated
            }
        }
    }
}

enum RunnerPhase<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    Construction {
        phase_index: usize,
        declaration: CandidateTracePhasePlan,
        outcome: RunnerPhaseOutcome,
    },
    LocalSearch {
        phase_index: usize,
        declaration: CandidateTracePhasePlan,
        local_search: RuntimeLocalSearch<S, V, DM, IDM>,
        outcome: RunnerPhaseOutcome,
    },
    Extension {
        phase_index: usize,
        declaration: CandidateTracePhasePlan,
        outcome: RunnerPhaseOutcome,
    },
    DefaultRuntime {
        phase_index: usize,
        local_search: Option<RuntimeLocalSearch<S, V, DM, IDM>>,
        local_search_declaration: Option<CandidateTracePhasePlan>,
        construction_execution: Option<DefaultRuntimeConstructionExecution>,
        outcome: RunnerPhaseOutcome,
    },
}

impl<S, V, DM, IDM, Extension> CompiledRuntimePhaseRunner<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    Extension: CustomSearchPhase<S>,
{
    /// Instantiates extensions in frozen configuration order, lowers every
    /// frozen local-search node once, and creates the one solve-owned provider
    /// resource set. It does not bind a list source or invoke a provider.
    pub(crate) fn try_new<Registry>(
        executor: &CompiledRuntimeExecutor<S, V, DM, IDM, Registry>,
    ) -> RuntimeBuildResult<Self>
    where
        Registry: RuntimeExtensionRegistry<S, V, DM, IDM, Phase = Extension>,
    {
        let graph = executor.graph();
        let solver_config = graph.config().clone();
        let mandatory_bindings = graph.default_bindings().clone();
        let provider_resources =
            ProviderExecutionResources::new(graph.context().model().runtime_provider_registry());
        let mut execution = executor.instantiate().map_err(map_preparation_error)?;
        assert_eq!(
            execution.phases.len(),
            graph.phases().len(),
            "prepared runtime phases must retain the frozen graph order"
        );
        let prepared_phases = std::mem::take(&mut execution.phases);
        let phases = prepared_phases
            .iter()
            .zip(graph.phases())
            .enumerate()
            .map(|(phase_index, (prepared, declared))| {
                lower_runner_phase(
                    phase_index,
                    prepared,
                    declared,
                    &solver_config,
                    graph.default_bindings(),
                )
            })
            .collect::<RuntimeBuildResult<Vec<_>>>()?;
        Ok(Self {
            execution,
            prepared_phases,
            phases,
            provider_resources,
            mandatory_bindings,
            mandatory_completion_published: false,
            terminal_notified: false,
        })
    }

    #[cfg(test)]
    pub(crate) fn phase_plan_for_test(&self, phase_index: usize) -> CandidateTracePhasePlan {
        self.phases[phase_index].candidate_trace_plan()
    }

    fn solve_phase<D, ProgressCb>(
        &mut self,
        index: usize,
        solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
    ) where
        D: Director<S>,
        ProgressCb: ProgressCallback<S>,
    {
        let phase_index = self.phases[index].phase_index();
        let (
            execution,
            prepared_phases,
            phases,
            provider_resources,
            mandatory_bindings,
            mandatory_completion_published,
        ) = (
            &mut self.execution,
            &mut self.prepared_phases,
            &mut self.phases,
            &mut self.provider_resources,
            &self.mandatory_bindings,
            &mut self.mandatory_completion_published,
        );
        let (prepared, runner_phase) = (&mut prepared_phases[index], &mut phases[index]);
        match (prepared, runner_phase) {
            (
                PreparedRuntimePhase::Construction(construction),
                RunnerPhase::Construction { outcome, .. },
            ) => {
                let construction_execution = super::execute_prepared_construction(
                    execution,
                    construction,
                    false,
                    solver_scope,
                )
                .unwrap_or_else(|error| panic_execution_error(error));
                if !construction_execution.ran() {
                    finalize_noop_construction(solver_scope);
                }
                *outcome = RunnerPhaseOutcome::from_construction(construction_execution);
                publish_if_mandatory_complete(
                    execution,
                    mandatory_bindings,
                    mandatory_completion_published,
                    phase_index,
                    solver_scope,
                )
                .unwrap_or_else(|error| panic_runtime_execution_error(error));
            }
            (
                PreparedRuntimePhase::LocalSearch(_),
                RunnerPhase::LocalSearch {
                    local_search,
                    outcome,
                    ..
                },
            ) => {
                if require_mandatory_completion(
                    execution,
                    mandatory_bindings,
                    mandatory_completion_published,
                    phase_index,
                    solver_scope,
                )
                .unwrap_or_else(|error| panic_runtime_execution_error(error))
                {
                    local_search.solve(provider_resources, solver_scope);
                    *outcome = RunnerPhaseOutcome::Executed;
                } else {
                    *outcome = RunnerPhaseOutcome::SkippedTerminated;
                }
            }
            (
                PreparedRuntimePhase::Extension(extension),
                RunnerPhase::Extension { outcome, .. },
            ) => {
                extension.solve(solver_scope);
                *outcome = RunnerPhaseOutcome::Executed;
                publish_if_mandatory_complete(
                    execution,
                    mandatory_bindings,
                    mandatory_completion_published,
                    phase_index,
                    solver_scope,
                )
                .unwrap_or_else(|error| panic_runtime_execution_error(error));
            }
            (
                PreparedRuntimePhase::DefaultRuntime(default),
                RunnerPhase::DefaultRuntime {
                    local_search,
                    construction_execution,
                    outcome,
                    ..
                },
            ) => {
                let record =
                    super::execute_prepared_default_construction(execution, default, solver_scope)
                        .unwrap_or_else(|error| panic_execution_error(error));
                let construction_outcome = record.outcome();
                if !record.ran_child_phase {
                    finalize_noop_construction(solver_scope);
                }
                *construction_execution = Some(record);
                let mandatory_complete = require_mandatory_completion(
                    execution,
                    mandatory_bindings,
                    mandatory_completion_published,
                    phase_index,
                    solver_scope,
                )
                .unwrap_or_else(|error| panic_runtime_execution_error(error));
                let local_search_ran = if mandatory_complete && !solver_scope.should_terminate() {
                    if let Some(local_search) = local_search {
                        local_search.solve(provider_resources, solver_scope);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                *outcome = if local_search_ran {
                    RunnerPhaseOutcome::Executed
                } else {
                    RunnerPhaseOutcome::from_construction(ConstructionExecution {
                        outcome: construction_outcome,
                    })
                };
            }
            _ => panic!("prepared runtime phase must retain its lowered runner variant"),
        }
    }

    fn final_phase_plan(&self) -> CandidateTracePhasePlan {
        CandidateTracePhasePlan::known(
            "solverforge.runtime.compiled",
            [("phase_count", self.phases.len().to_string())],
            self.phases
                .iter()
                .map(RunnerPhase::candidate_trace_plan)
                .collect(),
        )
    }
}

impl<S, V, DM, IDM> RunnerPhase<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
{
    fn phase_index(&self) -> usize {
        match self {
            Self::Construction { phase_index, .. }
            | Self::LocalSearch { phase_index, .. }
            | Self::Extension { phase_index, .. }
            | Self::DefaultRuntime { phase_index, .. } => *phase_index,
        }
    }

    fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        match self {
            Self::Construction {
                phase_index,
                declaration,
                outcome,
            }
            | Self::LocalSearch {
                phase_index,
                declaration,
                outcome,
                ..
            }
            | Self::Extension {
                phase_index,
                declaration,
                outcome,
            } => phase_with_outcome(*phase_index, outcome.trace_label(), declaration.clone()),
            Self::DefaultRuntime {
                phase_index,
                local_search_declaration,
                construction_execution,
                outcome,
                ..
            } => {
                let mut children = construction_execution
                    .as_ref()
                    .map(default_construction_plan)
                    .into_iter()
                    .collect::<Vec<_>>();
                if let Some(local_search) = local_search_declaration {
                    children.push(local_search.clone());
                }
                CandidateTracePhasePlan::known(
                    "solverforge.runtime.default_runtime",
                    [
                        ("outcome", outcome.trace_label().to_string()),
                        ("phase_index", phase_index.to_string()),
                    ],
                    children,
                )
            }
        }
    }
}

impl<S, V, DM, IDM, Extension> Debug for CompiledRuntimePhaseRunner<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    Extension: Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompiledRuntimePhaseRunner")
            .field("phase_count", &self.phases.len())
            .finish()
    }
}

impl<S, V, DM, IDM, Extension, D, ProgressCb> Phase<S, D, ProgressCb>
    for CompiledRuntimePhaseRunner<S, V, DM, IDM, Extension>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    Extension: CustomSearchPhase<S>,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        publish_if_mandatory_complete(
            &mut self.execution,
            &self.mandatory_bindings,
            &mut self.mandatory_completion_published,
            0,
            solver_scope,
        )
        .unwrap_or_else(|error| panic_runtime_execution_error(error));
        for index in 0..self.phases.len() {
            if solver_scope.should_terminate() {
                break;
            }
            self.solve_phase(index, solver_scope);
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "CompiledRuntime"
    }

    fn defers_initial_best_solution_publication(&self) -> bool {
        !self.mandatory_bindings.list_slots.is_empty()
            || self
                .mandatory_bindings
                .scalar_slots
                .iter()
                .any(|binding| !binding.assignment_owned && !binding.slot.allows_unassigned())
            || !self.mandatory_bindings.assignment_groups.is_empty()
    }

    fn on_solver_terminal(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        if self.terminal_notified {
            return;
        }
        self.terminal_notified = true;
        for (prepared, runner_phase) in self.prepared_phases.iter_mut().zip(&mut self.phases) {
            if let (PreparedRuntimePhase::Extension(extension), RunnerPhase::Extension { .. }) =
                (prepared, runner_phase)
            {
                extension.on_solver_terminal(solver_scope);
            }
        }
        solver_scope.finalize_candidate_trace_resolved_phase_plan(self.final_phase_plan());
        let phase_index = self
            .phases
            .last()
            .map(RunnerPhase::phase_index)
            .unwrap_or(0);
        require_mandatory_completion(
            &mut self.execution,
            &self.mandatory_bindings,
            &mut self.mandatory_completion_published,
            phase_index,
            solver_scope,
        )
        .unwrap_or_else(|error| panic_runtime_execution_error(error));
    }

    fn candidate_trace_plan(&self) -> CandidateTracePhasePlan {
        // Default construction resolves against the post-predecessor working
        // state, so no pre-solve plan can honestly claim its child sequence.
        // The terminal hook replaces this provisional marker exactly once.
        CandidateTracePhasePlan::opaque("solverforge.runtime.compiled.pending_resolution")
    }
}
