//! The one retained phase runner for a compiled runtime graph.
//!
//! It consumes only prepared graph nodes and lowered frozen selectors. In
//! particular, it never re-enters public selector builders, reconstructs a
//! construction model, or owns a second provider registry.

mod bench;
mod failure;
mod local_search;
mod lowering;
mod phase_plan;
mod problem_change;

use std::fmt::{self, Debug};

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::CustomSearchPhase;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::Phase;
use crate::runtime::compiler::{RuntimeGraphInput, RuntimeSource};
use crate::runtime::finalize_noop_construction;
use crate::runtime_build_error::RuntimeBuildResult;
use crate::scope::{ProgressCallback, SolverScope};
use crate::stats::CandidateTracePhasePlan;

use super::completion::{publish_if_mandatory_complete, require_mandatory_completion};
use super::local_search::ProviderExecutionResources;
use super::partitioned::{solve_partitioned_instance, RuntimeExtensionInstance, SourceExtension};
use super::{
    CompiledRuntimeExecutor, ConstructionExecution, DefaultRuntimeConstructionExecution,
    PreparedRuntimeExecution, PreparedRuntimePhase,
};
use crate::runtime::compiler::DefaultRuntimeBindings;
pub(crate) use failure::panic_runtime_execution_error;
use failure::{map_preparation_error, panic_execution_error};
use local_search::RuntimeLocalSearch;
use lowering::lower_runner_phase;

pub(crate) use bench::ScalarConstructionHandoff;
pub(crate) use failure::take_runtime_execution_failure;

/// One retained per-solve executor. A public configured entrypoint creates it
/// after graph compilation/preparation and hands it to the ordinary solver as
/// a single `Phase`; pause and resume happen inside that same object.
pub(crate) struct CompiledRuntimePhaseRunner<S, V, DM, IDM, H>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    H: RuntimeSource<S, V, DM, IDM>,
{
    /// The declaration the graph was compiled from, kept only when a
    /// partitioned phase compiles each partition's runner from it, so a
    /// solve without one does not hold a second copy of the search context.
    /// Boxed so the runner itself stays the size it is without one.
    source: Option<Box<H>>,
    execution: PreparedRuntimeExecution<S, V, DM, IDM, SourceExtension<S, V, DM, IDM, H>>,
    prepared_phases: Vec<PreparedRuntimePhase<S, V, DM, IDM, SourceExtension<S, V, DM, IDM, H>>>,
    phases: Vec<RunnerPhase<S, V, DM, IDM>>,
    provider_resources: ProviderExecutionResources<S>,
    mandatory_bindings: DefaultRuntimeBindings<S, V, DM, IDM>,
    mandatory_completion_published: bool,
    terminal_notified: bool,
    problem_change_passes: usize,
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

impl<S, V, DM, IDM, H> CompiledRuntimePhaseRunner<S, V, DM, IDM, H>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    H: RuntimeSource<S, V, DM, IDM>,
    RuntimeGraphInput<S, V, DM, IDM, H::Extensions>: Sync,
{
    /// Whether the runner kept its declaration for a partitioned phase.
    #[cfg(test)]
    pub(crate) fn keeps_declaration(&self) -> bool {
        self.source.is_some()
    }

    /// Instantiates extensions in frozen configuration order, lowers every
    /// frozen local-search node once, and creates the one solve-owned provider
    /// resource set. It does not bind a list source or invoke a provider.
    pub(crate) fn try_new(
        executor: CompiledRuntimeExecutor<S, V, DM, IDM, H>,
    ) -> RuntimeBuildResult<Self> {
        let solver_config = executor.graph().config().clone();
        let mut mandatory_bindings = executor.graph().default_bindings().clone();
        let provider_resources = ProviderExecutionResources::new(
            executor
                .graph()
                .context()
                .model()
                .runtime_provider_registry(),
        );
        let mut execution = executor.instantiate().map_err(map_preparation_error)?;
        let mut graph = executor.into_graph();
        assert_eq!(
            execution.phases.len(),
            graph.phases().len(),
            "prepared runtime phases must retain the frozen graph order"
        );
        let prepared_phases = std::mem::take(&mut execution.phases);
        let declared_phases = std::mem::take(&mut graph.phases);
        let phases = prepared_phases
            .iter()
            .zip(declared_phases)
            .enumerate()
            .map(|(phase_index, (prepared, declared))| {
                lower_runner_phase(
                    phase_index,
                    prepared,
                    declared,
                    &solver_config,
                    &mut mandatory_bindings,
                )
            })
            .collect::<RuntimeBuildResult<Vec<_>>>()?;
        let partitioned = prepared_phases.iter().any(|phase| {
            matches!(
                phase,
                PreparedRuntimePhase::Extension(RuntimeExtensionInstance::Partitioned(_))
            )
        });
        Ok(Self {
            source: partitioned.then(|| Box::new(graph.source)),
            execution,
            prepared_phases,
            phases,
            provider_resources,
            mandatory_bindings,
            mandatory_completion_published: false,
            terminal_notified: false,
            problem_change_passes: 0,
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
            source,
            execution,
            prepared_phases,
            phases,
            provider_resources,
            mandatory_bindings,
            mandatory_completion_published,
        ) = (
            &self.source,
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
                match extension {
                    RuntimeExtensionInstance::Custom(phase) => phase.solve(solver_scope),
                    RuntimeExtensionInstance::Partitioned(instance) => {
                        let source = source
                            .as_ref()
                            .expect("a runner with a partitioned phase keeps its declaration");
                        solve_partitioned_instance(instance, source.input(), solver_scope);
                    }
                }
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
        let problem_change_passes = (self.problem_change_passes > 0).then(|| {
            (
                "problem_change_passes",
                self.problem_change_passes.to_string(),
            )
        });
        CandidateTracePhasePlan::known(
            "solverforge.runtime.compiled",
            std::iter::once(("phase_count", self.phases.len().to_string()))
                .chain(problem_change_passes),
            self.phases
                .iter()
                .map(RunnerPhase::candidate_trace_plan)
                .collect(),
        )
    }
}

impl<S, V, DM, IDM, H> Debug for CompiledRuntimePhaseRunner<S, V, DM, IDM, H>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    H: RuntimeSource<S, V, DM, IDM>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CompiledRuntimePhaseRunner")
            .field("phase_count", &self.phases.len())
            .finish()
    }
}

impl<S, V, DM, IDM, H, D, ProgressCb> Phase<S, D, ProgressCb>
    for CompiledRuntimePhaseRunner<S, V, DM, IDM, H>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    H: RuntimeSource<S, V, DM, IDM> + Send,
    RuntimeGraphInput<S, V, DM, IDM, H::Extensions>: Sync,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<'_, S, D, ProgressCb>) {
        self.solve_passes(solver_scope);
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
        solver_scope.close_problem_changes();
        for (prepared, runner_phase) in self.prepared_phases.iter_mut().zip(&mut self.phases) {
            if let (
                PreparedRuntimePhase::Extension(RuntimeExtensionInstance::Custom(phase)),
                RunnerPhase::Extension { .. },
            ) = (prepared, runner_phase)
            {
                phase.on_solver_terminal(solver_scope);
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
