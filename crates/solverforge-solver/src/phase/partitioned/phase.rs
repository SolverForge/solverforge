// PartitionedSearchPhase implementation.

use std::fmt::Debug;
use std::marker::PhantomData;

use rayon::prelude::*;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::phase::Phase;
use crate::scope::ProgressCallback;
use crate::scope::SolverScope;

use super::child_phases::ChildPhases;
use super::config::PartitionedSearchConfig;
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
/// * `D` - The main solver's score director type
/// * `PD` - The score director type for partition solvers
/// * `Part` - The partitioner type (implements `SolutionPartitioner<S>`)
/// * `SDF` - The score director factory function type
/// * `PF` - The phase factory function type
/// * `CP` - The child phases type (tuple of phases)
pub struct PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: Director<S>,
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

    _marker: PhantomData<fn(S, D, PD, CP)>,
}

impl<S, D, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: Director<S>,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    pub fn new(partitioner: Part, score_director_factory: SDF, phase_factory: PF) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config: PartitionedSearchConfig::default(),
            _marker: PhantomData,
        }
    }

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

impl<S, D, PD, Part, SDF, PF, CP> Debug for PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: Director<S>,
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
    for PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
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
        // Get the current solution
        let solution = solver_scope.score_director().working_solution().clone();

        // Partition the solution
        let partitions = self.partitioner.partition(&solution);
        let partition_count = partitions.len();

        if partition_count == 0 {
            return;
        }

        // Determine thread count
        let thread_count = self.config.thread_count.resolve(partition_count);

        if self.config.log_progress {
            tracing::info!(event = "phase_start", phase = "PartitionedSearch",);
        }

        // Solve partitions - parallel when multiple, sequential when single
        let solved_partitions: Vec<S> = if thread_count == 1 || partition_count == 1 {
            partitions
                .into_iter()
                .map(|p| self.solve_partition(p))
                .collect()
        } else {
            partitions
                .into_par_iter()
                .map(|partition| {
                    let director = (self.score_director_factory)(partition);
                    let mut solver_scope = SolverScope::new(director);
                    let mut phases = (self.phase_factory)();
                    phases.solve_all(&mut solver_scope);
                    solver_scope.take_best_or_working_solution()
                })
                .collect()
        };

        // Merge the solved partitions
        let merged = self.partitioner.merge(&solution, solved_partitions);

        // Update the working solution with the merged result
        let director = solver_scope.score_director_mut();

        // Replace working solution with merged result
        let working = director.working_solution_mut();
        *working = merged;

        // Calculate the score of the merged solution
        solver_scope.calculate_score();

        // Update best solution
        solver_scope.update_best_solution();

        if self.config.log_progress {
            if let Some(score) = solver_scope.best_score() {
                tracing::info!(
                    event = "phase_end",
                    phase = "PartitionedSearch",
                    score = %format!("{:?}", score),
                );
            }
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "PartitionedSearch"
    }
}

impl<S, D, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: Director<S>,
    PD: Director<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    // Solves a single partition and returns the solved solution.
    fn solve_partition(&self, partition: S) -> S {
        // Create score director for this partition
        let director = (self.score_director_factory)(partition);

        // Create solver scope
        let mut solver_scope = SolverScope::new(director);

        // Create and run child phases
        let mut phases = (self.phase_factory)();
        phases.solve_all(&mut solver_scope);

        // Return the best solution (or working solution if no best)
        solver_scope.take_best_or_working_solution()
    }
}

#[cfg(test)]
mod tests {
    use super::super::partitioner::ThreadCount;
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PartitionedSearchConfig::default();
        assert_eq!(config.thread_count, ThreadCount::Auto);
        assert!(!config.log_progress);
    }
}
