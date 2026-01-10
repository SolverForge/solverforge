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
use std::marker::PhantomData;
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
    D: ScoreDirector<S>,
    PD: ScoreDirector<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    /// The partitioner that splits and merges solutions.
    partitioner: Part,

    /// Factory for creating score directors for each partition.
    score_director_factory: SDF,

    /// Factory for creating child phases for each partition.
    phase_factory: PF,

    /// Configuration for this phase.
    config: PartitionedSearchConfig,

    _marker: PhantomData<fn(S, D, PD, CP)>,
}

impl<S, D, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    PD: ScoreDirector<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    /// Creates a new partitioned search phase.
    pub fn new(partitioner: Part, score_director_factory: SDF, phase_factory: PF) -> Self {
        Self {
            partitioner,
            score_director_factory,
            phase_factory,
            config: PartitionedSearchConfig::default(),
            _marker: PhantomData,
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
            _marker: PhantomData,
        }
    }
}

impl<S, D, PD, Part, SDF, PF, CP> Debug for PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    PD: ScoreDirector<S>,
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

impl<S, D, PD, Part, SDF, PF, CP> Phase<S, D> for PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution + 'static,
    D: ScoreDirector<S>,
    PD: ScoreDirector<S> + 'static,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD> + Send,
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
                .map(|p| self.solve_partition(p))
                .collect()
        } else {
            // Parallel execution
            let results: Arc<Mutex<Vec<Option<S>>>> =
                Arc::new(Mutex::new(vec![None; partition_count]));

            thread::scope(|s| {
                for (i, partition) in partitions.into_iter().enumerate() {
                    let results = Arc::clone(&results);
                    let sdf = &self.score_director_factory;
                    let pf = &self.phase_factory;

                    s.spawn(move || {
                        let director = sdf(partition);
                        let mut solver_scope = SolverScope::new(director);
                        let mut phases = pf();
                        phases.solve_all(&mut solver_scope);
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

impl<S, D, PD, Part, SDF, PF, CP> PartitionedSearchPhase<S, D, PD, Part, SDF, PF, CP>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
    PD: ScoreDirector<S>,
    Part: SolutionPartitioner<S>,
    SDF: Fn(S) -> PD + Send + Sync,
    PF: Fn() -> CP + Send + Sync,
    CP: ChildPhases<S, PD>,
{
    /// Solves a single partition and returns the solved solution.
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

/// Trait for child phases that can solve a partition.
///
/// Implemented for tuples of phases via macro.
pub trait ChildPhases<S, D>
where
    S: PlanningSolution,
    D: ScoreDirector<S>,
{
    /// Runs all child phases on the solver scope.
    fn solve_all(&mut self, solver_scope: &mut SolverScope<S, D>);
}

// Implement ChildPhases for tuples using macro
macro_rules! impl_child_phases_tuple {
    ($($idx:tt: $P:ident),+) => {
        impl<S, D, $($P),+> ChildPhases<S, D> for ($($P,)+)
        where
            S: PlanningSolution,
            D: ScoreDirector<S>,
            $($P: Phase<S, D>,)+
        {
            fn solve_all(&mut self, solver_scope: &mut SolverScope<S, D>) {
                $(
                    self.$idx.solve(solver_scope);
                )+
            }
        }
    };
}

impl_child_phases_tuple!(0: P0);
impl_child_phases_tuple!(0: P0, 1: P1);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6);
impl_child_phases_tuple!(0: P0, 1: P1, 2: P2, 3: P3, 4: P4, 5: P5, 6: P6, 7: P7);

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
}
