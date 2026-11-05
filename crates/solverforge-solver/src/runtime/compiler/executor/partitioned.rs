//! The configured runtime's `partitioned_search` phase: the registered
//! partitioner splits the working solution, each partition is solved by a
//! runner compiled from the phase's child configuration, and the solved
//! partitions are merged back.

use std::fmt::Debug;

use solverforge_config::{SolverConfig, TerminationConfig};
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::{ParseableScore, Score};
use solverforge_scoring::Director;

use crate::builder::RuntimeExtensionRegistry;
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;
use crate::phase::partitioned::config::PartitionedSearchConfig;
use crate::phase::partitioned::execution::solve_partitioned;
use crate::phase::Phase;
use crate::runtime::compiler::{
    compile_runtime_graph, partition_child_config, RuntimeGraphInput, RuntimeSource,
};
use crate::runtime_build_error::RuntimeBuildError;
use crate::scope::{ProgressCallback, SolverScope};

use super::runner::panic_runtime_execution_error;
use super::{CompiledRuntimeExecutor, CompiledRuntimePhaseRunner};

/// One instantiated extension phase of a compiled runner.
#[expect(
    clippy::large_enum_variant,
    reason = "prepared phases remain value-owned without execution-time indirection"
)]
pub(crate) enum RuntimeExtensionInstance<Phase, Partitioner> {
    /// A registered custom phase.
    Custom(Phase),
    /// A `partitioned_search` phase with its registered partitioner.
    Partitioned(PartitionedInstance<Partitioner>),
}

/// The extension instance type of a runner over source `H`.
pub(crate) type SourceExtension<S, V, DM, IDM, H> =
    RuntimeExtensionInstance<
        <<H as RuntimeSource<S, V, DM, IDM>>::Extensions as RuntimeExtensionRegistry<
            S,
            V,
            DM,
            IDM,
        >>::Phase,
        <<H as RuntimeSource<S, V, DM, IDM>>::Extensions as RuntimeExtensionRegistry<
            S,
            V,
            DM,
            IDM,
        >>::Partitioner,
    >;

/// A `partitioned_search` phase instantiated for one solve.
pub(crate) struct PartitionedInstance<Partitioner> {
    partitioner: Partitioner,
    config: PartitionedSearchConfig,
    /// The phase's termination: its time and step count limits bound all
    /// partitions together.
    termination: Option<TerminationConfig>,
    /// The configuration each partition is solved with.
    child_config: SolverConfig,
}

impl<Partitioner> PartitionedInstance<Partitioner> {
    pub(crate) fn new(
        partitioner: Partitioner,
        config: &solverforge_config::PartitionedSearchConfig,
        parent: &SolverConfig,
    ) -> Self {
        Self {
            partitioner,
            config: PartitionedSearchConfig::from_serialized(config),
            termination: config.termination.clone(),
            child_config: partition_child_config(parent, config),
        }
    }
}

impl<Partitioner: Debug> Debug for PartitionedInstance<Partitioner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionedInstance")
            .field("partitioner", &self.partitioner)
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

/// Runs one partitioned phase. Each partition gets a director forked from
/// the solve's own director over the partition and a runner compiled from
/// the child configuration over the parent's declaration; the compilation
/// cannot fail here because the graph compiler already compiled the same
/// child configuration.
pub(super) fn solve_partitioned_instance<S, V, DM, IDM, E, D, ProgressCb>(
    instance: &PartitionedInstance<E::Partitioner>,
    input: &RuntimeGraphInput<S, V, DM, IDM, E>,
    solver_scope: &mut SolverScope<'_, S, D, ProgressCb>,
) where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score + Copy + Ord + ParseableScore,
    V: Clone + PartialEq + Into<usize> + Send + Sync + Debug + 'static,
    DM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    IDM: Clone + Send + Sync + Debug + CrossEntityDistanceMeter<S> + 'static,
    E: RuntimeExtensionRegistry<S, V, DM, IDM>,
    RuntimeGraphInput<S, V, DM, IDM, E>: Sync,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let solve_children = |child_scope: &mut SolverScope<'_, S, D>| {
        let graph = compile_runtime_graph(&instance.child_config, input).unwrap_or_else(|error| {
            let message = error.to_string();
            panic_runtime_execution_error(RuntimeBuildError::Compilation {
                path: error.path,
                message,
            })
        });
        let mut runner = CompiledRuntimePhaseRunner::try_new(CompiledRuntimeExecutor::new(graph))
            .unwrap_or_else(|error| panic_runtime_execution_error(error));
        Phase::solve(&mut runner, child_scope);
    };
    solve_partitioned(
        solver_scope,
        &instance.partitioner,
        &instance.config,
        instance.termination.as_ref(),
        |director: &D, partition| {
            director.fork_for_solution(partition).expect(
                "partitioned search requires a director built with a constraint-set constructor",
            )
        },
        &solve_children,
    );
}
