/* Partitioned search phase for parallel solving.

Partitioned search splits a large problem into independent sub-problems
(partitions) that can be solved in parallel, then merges the results.

# Usage

1. Define a partitioner that knows how to split and merge your solution type
2. Create a partitioned search phase with child phases
3. The phase will partition the solution, solve each partition, and merge

# Example

```
use solverforge_solver::phase::partitioned::{PartitionedSearchConfig, ThreadCount};

let config = PartitionedSearchConfig {
thread_count: ThreadCount::Specific(4),
log_progress: true,
};
```
*/

mod child_phases;
mod config;
mod partitioner;
mod phase;

pub use child_phases::ChildPhases;
pub use config::PartitionedSearchConfig;
pub use partitioner::{FunctionalPartitioner, SolutionPartitioner, ThreadCount};
pub use phase::PartitionedSearchPhase;
