//! Partitioned search phase for parallel solving.
//!
//! Partitioned search splits a large problem into independent sub-problems
//! (partitions) that can be solved in parallel, then merges the results.
//!
//! # Usage
//!
//! 1. Define a partitioner that knows how to split and merge your solution type
//! 2. Create a partitioned search phase with child phases
//! 3. The phase will partition the solution, solve each partition, and merge
//!
//! # Example
//!
//! ```
//! use solverforge_solver::phase::partitioned::{PartitionedSearchConfig, ThreadCount};
//!
//! let config = PartitionedSearchConfig {
//!     thread_count: ThreadCount::Specific(4),
//!     log_progress: true,
//! };
//! ```

mod partitioner;

use std::fmt::Debug;
use std::sync::{Arc, Mutex};
use std::thread;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

use crate::phase::Phase;
use crate::scope::SolverScope;

pub use partitioner::{FunctionalPartitioner, SolutionPartitioner, ThreadCount};

/// Configuration for partitioned search phase.
#[derive(Debug, Clone)]
pub struct PartitionedSearchConfig {
    /// Thread count configuration.
    pub thread_count: ThreadCount,
    /// Whether to log partition progress.
    pub log_progress: bool,
}

impl Default for PartitionedSearchConfig {
    fn default() -> Self {
        Self {
            thread_count: ThreadCount::Auto,
            log_progress: false,
        }
    }
}

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
/// * `S` - The planning solution type
/// * `D` - The score director type
/// * `Part` - The partitioner type
/// * `SDF` - The score director factory type
/// * `PF` - The phase factory type
pub struct PartitionedSearchPhase<S, D, Part, SDF, PF>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> D + Send + Sync,
    PF: Fn() -> Vec<Box<dyn Phase<S, D>>> + Send + Sync,
{
    /// The partitioner that splits and merges solutions.
    partitioner: Part,

    /// Factory for creating score directors for each partition.
    score_director_factory: SDF,

    /// Factory for creating child phases for each partition.
    phase_factory: PF,

    /// Configuration for this phase.
    config: PartitionedSearchConfig,

    _phantom: std::marker::PhantomData<(S, D)>,
}

impl<S, D, Part, SDF, PF> PartitionedSearchPhase<S, D, Part, SDF, PF>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> D + Send + Sync,
    PF: Fn() -> Vec<Box<dyn Phase<S, D>>> + Send + Sync,
{
    /// Creates a new partitioned search phase.
    pub fn new(partitioner: Part, score_director_factory: SDF, phase_factory: PF) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config: PartitionedSearchConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Creates a partitioned search phase with custom configuration.
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
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S, D, Part, SDF, PF> Debug for PartitionedSearchPhase<S, D, Part, SDF, PF>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> D + Send + Sync,
    PF: Fn() -> Vec<Box<dyn Phase<S, D>>> + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionedSearchPhase")
            .field("partitioner", &self.partitioner)
            .field("config", &self.config)
            .finish()
    }
}

impl<S, D, Part, SDF, PF> Phase<S, D> for PartitionedSearchPhase<S, D, Part, SDF, PF>
where
    S: PlanningSolution + 'static,
    D: ScoreDirector<S> + 'static,
    Part: SolutionPartitioner<S> + Send,
    SDF: Fn(S) -> D + Send + Sync + Clone,
    PF: Fn() -> Vec<Box<dyn Phase<S, D>>> + Send + Sync + Clone,
{
    fn solve(&mut self, solver_scope: &mut SolverScope<S, D>) {
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
            println!(
                "[PartitionedSearch] Solving {} partitions with {} threads",
                partition_count, thread_count
            );
        }

        // Solve partitions
        let solved_partitions: Vec<S> = if thread_count == 1 || partition_count == 1 {
            // Sequential execution
            partitions
                .into_iter()
                .map(|partition| {
                    let director = (self.score_director_factory)(partition);
                    let mut solver_scope = SolverScope::new(director);
                    let mut phases = (self.phase_factory)();
                    for phase in phases.iter_mut() {
                        phase.solve(&mut solver_scope);
                    }
                    solver_scope.take_best_or_working_solution()
                })
                .collect()
        } else {
            // Parallel execution
            let results: Arc<Mutex<Vec<Option<S>>>> =
                Arc::new(Mutex::new(vec![None; partition_count]));

            let sdf = self.score_director_factory.clone();
            let pf = self.phase_factory.clone();

            thread::scope(|s| {
                for (i, partition) in partitions.into_iter().enumerate() {
                    let results = Arc::clone(&results);
                    let sdf = sdf.clone();
                    let pf = pf.clone();

                    s.spawn(move || {
                        let director = sdf(partition);
                        let mut solver_scope = SolverScope::new(director);
                        let mut phases = pf();
                        for phase in phases.iter_mut() {
                            phase.solve(&mut solver_scope);
                        }
                        let solved = solver_scope.take_best_or_working_solution();
                        let mut r = results.lock().unwrap();
                        r[i] = Some(solved);
                    });
                }
            });

            // Extract results
            let results = Arc::try_unwrap(results)
                .unwrap_or_else(|_| panic!("All threads should be done"))
                .into_inner()
                .unwrap();

            results.into_iter().map(|opt| opt.unwrap()).collect()
        };

        // Merge the solved partitions
        let merged = self.partitioner.merge(&solution, solved_partitions);

        // Update the working solution with the merged result
        let director = solver_scope.score_director_mut();
        let working = director.working_solution_mut();
        *working = merged;

        // Calculate the score of the merged solution
        solver_scope.calculate_score();

        // Update best solution
        solver_scope.update_best_solution();

        if self.config.log_progress {
            if let Some(score) = solver_scope.best_score() {
                println!(
                    "[PartitionedSearch] Completed with merged score: {:?}",
                    score
                );
            }
        }
    }

    fn phase_type_name(&self) -> &'static str {
        "PartitionedSearch"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = PartitionedSearchConfig::default();
        assert_eq!(config.thread_count, ThreadCount::Auto);
        assert!(!config.log_progress);
    }
}
