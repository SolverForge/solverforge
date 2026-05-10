// Partitioned search phase configuration.

use solverforge_config::MoveThreadCount;

use super::partitioner::ThreadCount;

// Configuration for partitioned search phase.
#[derive(Debug, Clone)]
pub struct PartitionedSearchConfig {
    // Thread count configuration.
    pub thread_count: ThreadCount,
    // Whether to log partition progress.
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

impl PartitionedSearchConfig {
    pub fn from_serialized(config: &solverforge_config::PartitionedSearchConfig) -> Self {
        Self {
            thread_count: match config.thread_count {
                MoveThreadCount::Auto => ThreadCount::Auto,
                MoveThreadCount::None => ThreadCount::Specific(1),
                MoveThreadCount::Count(count) => ThreadCount::Specific(count.max(1)),
            },
            log_progress: config.log_progress,
        }
    }
}
