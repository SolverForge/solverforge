/* Solution partitioner for dividing problems into independent sub-problems.

Partitioners split a large problem into smaller pieces that can be
solved independently (potentially in parallel), then merged back together.
*/

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

/// Splits a solution into independent partitions for parallel solving.
///
/// Each partition should be solvable independently without affecting
/// the correctness of other partitions. The partitioner must also be
/// able to merge the solved partitions back into a complete solution.
///
/// # Type Parameters
///
/// - `S`: The planning solution type
///
/// # Example
///
/// For a school timetabling problem, a natural partitioning might be
/// by room or by time period, where each partition contains lessons
/// that don't interact with lessons in other partitions.
pub trait SolutionPartitioner<S: PlanningSolution>: Send + Sync + Debug {
    /* Splits the solution into independent partitions.

    Each returned solution should be a subset of the original that
    can be optimized independently. The union of all partitions should
    cover all entities in the original solution.

    # Arguments

    * `solution` - The solution to partition

    # Returns

    A vector of partial solutions, one per partition.
    */
    fn partition(&self, solution: &S) -> Vec<S>;

    /* Merges solved partitions back into a complete solution.

    This is called after all partitions have been solved to combine
    them into the final result.

    # Arguments

    * `original` - The original unpartitioned solution
    * `partitions` - The solved partition solutions

    # Returns

    The merged complete solution.
    */
    fn merge(&self, original: &S, partitions: Vec<S>) -> S;

    /* Returns the recommended number of partitions.

    This can be used by the partitioned search phase to determine
    how many threads to use. Returns `None` if no recommendation.
    */
    fn recommended_partition_count(&self) -> Option<usize> {
        None
    }
}

/// A simple partitioner that creates a specified number of partitions.
///
/// This is a reference implementation that can be customized via
/// closures for the actual partitioning and merging logic.
pub struct FunctionalPartitioner<S, PF, MF>
where
    S: PlanningSolution,
    PF: Fn(&S) -> Vec<S> + Send + Sync,
    MF: Fn(&S, Vec<S>) -> S + Send + Sync,
{
    partition_fn: PF,
    merge_fn: MF,
    recommended_count: Option<usize>,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S, PF, MF> FunctionalPartitioner<S, PF, MF>
where
    S: PlanningSolution,
    PF: Fn(&S) -> Vec<S> + Send + Sync,
    MF: Fn(&S, Vec<S>) -> S + Send + Sync,
{
    pub fn new(partition_fn: PF, merge_fn: MF) -> Self {
        Self {
            partition_fn,
            merge_fn,
            recommended_count: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_recommended_count(mut self, count: usize) -> Self {
        self.recommended_count = Some(count);
        self
    }
}

impl<S, PF, MF> Debug for FunctionalPartitioner<S, PF, MF>
where
    S: PlanningSolution,
    PF: Fn(&S) -> Vec<S> + Send + Sync,
    MF: Fn(&S, Vec<S>) -> S + Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionalPartitioner")
            .field("recommended_count", &self.recommended_count)
            .finish()
    }
}

impl<S, PF, MF> SolutionPartitioner<S> for FunctionalPartitioner<S, PF, MF>
where
    S: PlanningSolution,
    PF: Fn(&S) -> Vec<S> + Send + Sync,
    MF: Fn(&S, Vec<S>) -> S + Send + Sync,
{
    fn partition(&self, solution: &S) -> Vec<S> {
        (self.partition_fn)(solution)
    }

    fn merge(&self, original: &S, partitions: Vec<S>) -> S {
        (self.merge_fn)(original, partitions)
    }

    fn recommended_partition_count(&self) -> Option<usize> {
        self.recommended_count
    }
}

// Thread count configuration for partitioned search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThreadCount {
    // Automatically determine based on available CPU cores.
    #[default]
    Auto,
    // Use all available CPU cores.
    Unlimited,
    // Use a specific number of threads.
    Specific(usize),
}

impl ThreadCount {
    /// Resolves the thread count to an actual number.
    ///
    /// # Arguments
    ///
    /// * `partition_count` - Number of partitions to process
    ///
    /// # Returns
    ///
    /// The number of threads to use.
    pub fn resolve(&self, partition_count: usize) -> usize {
        match self {
            ThreadCount::Auto => {
                let cpus = std::thread::available_parallelism()
                    .map(|p| p.get())
                    .unwrap_or(1);
                std::cmp::min(cpus, partition_count)
            }
            ThreadCount::Unlimited => std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1),
            ThreadCount::Specific(n) => std::cmp::min(*n, partition_count),
        }
    }
}

impl std::fmt::Display for ThreadCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadCount::Auto => write!(f, "Auto"),
            ThreadCount::Unlimited => write!(f, "Unlimited"),
            ThreadCount::Specific(n) => write!(f, "{}", n),
        }
    }
}

#[cfg(test)]
#[path = "partitioner_tests.rs"]
mod tests;
