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

/// A factory function for creating score directors.
pub type ScoreDirectorFactory<S> = Arc<dyn Fn(S) -> Box<dyn ScoreDirector<S>> + Send + Sync>;

/// A factory function for creating phases.
pub type PhaseFactory<S> = Arc<dyn Fn() -> Vec<Box<dyn Phase<S>>> + Send + Sync>;

/// Partitioned search phase that solves partitions in parallel.
///
/// This phase:
/// 1. Partitions the solution using the provided partitioner
/// 2. Creates a solver for each partition
/// 3. Runs child phases on each partition in parallel
/// 4. Merges the solved partitions back together
///
/// Each partition runs independently with its own solver scope.
pub struct PartitionedSearchPhase<S: PlanningSolution> {
    /// The partitioner that splits and merges solutions.
    partitioner: Box<dyn SolutionPartitioner<S>>,

    /// Factory for creating score directors for each partition.
    score_director_factory: ScoreDirectorFactory<S>,

    /// Factory for creating child phases for each partition.
    phase_factory: PhaseFactory<S>,

    /// Configuration for this phase.
    config: PartitionedSearchConfig,
}

impl<S: PlanningSolution> PartitionedSearchPhase<S> {
    /// Creates a new partitioned search phase.
    pub fn new(
        partitioner: Box<dyn SolutionPartitioner<S>>,
        score_director_factory: ScoreDirectorFactory<S>,
        phase_factory: PhaseFactory<S>,
    ) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config: PartitionedSearchConfig::default(),
        }
    }

    /// Creates a partitioned search phase with custom configuration.
    pub fn with_config(
        partitioner: Box<dyn SolutionPartitioner<S>>,
        score_director_factory: ScoreDirectorFactory<S>,
        phase_factory: PhaseFactory<S>,
        config: PartitionedSearchConfig,
    ) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config,
        }
    }

    /// Solves a single partition and returns the solved solution.
    fn solve_partition(
        partition: S,
        score_director_factory: &ScoreDirectorFactory<S>,
        phase_factory: &PhaseFactory<S>,
    ) -> S {
        // Create score director for this partition
        let director = (score_director_factory)(partition);

        // Create solver scope
        let mut solver_scope = SolverScope::new(director);

        // Create and run child phases
        let mut phases = (phase_factory)();
        for phase in phases.iter_mut() {
            phase.solve(&mut solver_scope);
        }

        // Return the best solution (or working solution if no best)
        solver_scope.take_best_or_working_solution()
    }
}

impl<S: PlanningSolution> Debug for PartitionedSearchPhase<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PartitionedSearchPhase")
            .field("partitioner", &self.partitioner)
            .field("config", &self.config)
            .finish()
    }
}

impl<S: PlanningSolution + 'static> Phase<S> for PartitionedSearchPhase<S> {
    fn solve(&mut self, solver_scope: &mut SolverScope<S>) {
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

        // Clone factories for threads
        let score_director_factory = Arc::clone(&self.score_director_factory);
        let phase_factory = Arc::clone(&self.phase_factory);

        // Solve partitions
        let solved_partitions: Vec<S> = if thread_count == 1 || partition_count == 1 {
            // Sequential execution
            partitions
                .into_iter()
                .map(|p| Self::solve_partition(p, &score_director_factory, &phase_factory))
                .collect()
        } else {
            // Parallel execution
            let results: Arc<Mutex<Vec<Option<S>>>> =
                Arc::new(Mutex::new(vec![None; partition_count]));

            thread::scope(|s| {
                for (i, partition) in partitions.into_iter().enumerate() {
                    let results = Arc::clone(&results);
                    let sdf = Arc::clone(&score_director_factory);
                    let pf = Arc::clone(&phase_factory);

                    s.spawn(move || {
                        let solved = Self::solve_partition(partition, &sdf, &pf);
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
        // We need to calculate the score and update best solution
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
    use solverforge_core::score::SimpleScore;

    #[derive(Clone, Debug)]
    struct TestSolution {
        values: Vec<i32>,
        score: Option<SimpleScore>,
    }

    impl PlanningSolution for TestSolution {
        type Score = SimpleScore;

        fn score(&self) -> Option<Self::Score> {
            self.score
        }

        fn set_score(&mut self, score: Option<Self::Score>) {
            self.score = score;
        }
    }

    #[test]
    fn test_config_default() {
        let config = PartitionedSearchConfig::default();
        assert_eq!(config.thread_count, ThreadCount::Auto);
        assert!(!config.log_progress);
    }

    #[test]
    fn test_phase_type_name() {
        let test_solution = TestSolution {
            values: vec![1, 2, 3],
            score: None,
        };
        assert_eq!(test_solution.values.len(), 3);

        let partitioner = Box::new(FunctionalPartitioner::new(
            |s: &TestSolution| vec![s.clone()],
            |_, partitions| partitions.into_iter().next().unwrap(),
        ));

        let sdf: ScoreDirectorFactory<TestSolution> = Arc::new(|_s| {
            panic!("Should not be called in this test");
        });

        let pf: PhaseFactory<TestSolution> = Arc::new(Vec::new);

        let phase = PartitionedSearchPhase::new(partitioner, sdf, pf);
        assert_eq!(phase.phase_type_name(), "PartitionedSearch");
    }

    #[test]
    fn test_phase_debug() {
        let partitioner = Box::new(FunctionalPartitioner::new(
            |s: &TestSolution| vec![s.clone()],
            |_, partitions| partitions.into_iter().next().unwrap(),
        ));

        let sdf: ScoreDirectorFactory<TestSolution> = Arc::new(|_s| {
            panic!("Should not be called in this test");
        });

        let pf: PhaseFactory<TestSolution> = Arc::new(Vec::new);

        let phase = PartitionedSearchPhase::new(partitioner, sdf, pf);
        let debug = format!("{:?}", phase);

        assert!(debug.contains("PartitionedSearchPhase"));
        assert!(debug.contains("FunctionalPartitioner"));
    }
}
