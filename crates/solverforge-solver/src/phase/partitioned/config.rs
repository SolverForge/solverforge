//! Partitioned search phase configuration.

use super::partitioner::ThreadCount;

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
