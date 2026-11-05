//! PartitionedSearchPhase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::SolverScope;

use super::child_phases::ChildPhases;
use super::config::PartitionedSearchConfig;
use super::execution::solve_partitioned;
use super::partitioner::SolutionPartitioner;

/// Partitioned search phase that solves partitions in parallel.
///
/// This phase:
/// 1. Partitions the solution using the provided partitioner
/// 2. Creates a solver for each partition
/// 3. Runs child phases on each partition in parallel
/// 4. Merges the solved partitions back together
///
/// Each partition runs independently with its own solver scope.
///
/// # Type Parameters
///
/// * `S` - The solution type
/// * `PD` - The score director type for partition solvers
/// * `Part` - The partitioner type (implements `SolutionPartitioner<S>`)
/// * `SDF` - The score director factory function type
/// * `PF` - The phase factory function type
/// * `CP` - The child phases type (tuple of phases)
pub struct PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    // The partitioner that splits and merges solutions.
    partitioner: Part,

    // Factory for creating score directors for each partition.
    score_director_factory: SDF,

    // Factory for creating child phases for each partition.
    phase_factory: PF,

    // Configuration for this phase.
    config: PartitionedSearchConfig,

    _marker: PhantomData<(fn() -> S, fn() -> PD, fn() -> CP)>,
}

impl<S, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    /// Creates the phase with the default configuration.
    pub fn new(partitioner: Part, score_director_factory: SDF, phase_factory: PF) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config: PartitionedSearchConfig::default(),
            _marker: PhantomData,
        }
    }

    /// Creates the phase with `config`.
    pub fn with_config(
        partitioner: Part,
        score_director_factory: SDF,
        phase_factory: PF,
        config: PartitionedSearchConfig,
    ) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config,
            _marker: PhantomData,
        }
    }
}

impl<S, PD, Part, SDF, PF, CP> Debug for PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    PD: Director<S>,
    Part: SolutionPartitioner<S> + Debug,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionedSearchPhase")
            .field("partitioner", &self.partitioner)
            .field("config", &self.config)
            .finish()
    }
}

impl<S, D, BestCb, PD, Part, SDF, PF, CP> Phase<S, D, BestCb>
    for PartitionedSearchPhase<S, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
    PD: Director<S> + 'static,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD> + Send,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D, BestCb>) {
        solve_partitioned(
            solver_scope,
            &self.partitioner,
            &self.config,
            |_: &D, partition| (self.score_director_factory)(partition),
            &|child_scope: &mut SolverScope<'_, S, PD>| {
                (self.phase_factory)().solve_all(child_scope);
            },
        );
    }

    fn phase_type_name(&self) -> &'static str {
        "PartitionedSearch"
    }
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/problem_change.rs"]
mod problem_change_tests;
