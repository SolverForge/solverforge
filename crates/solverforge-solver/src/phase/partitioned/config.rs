//! Partitioned search phase configuration.

use std::num::NonZeroUsize;

use solverforge_config::MoveThreadCount;

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

impl PartitionedSearchConfig {
    /// Converts the TOML configuration: thread count `auto` is one thread per
    /// partition up to the available CPU cores and the partitioner's
    /// recommended partition count, `none` is one thread, and an
    /// explicit count is at most that many threads.
    pub fn from_serialized(config: &solverforge_config::PartitionedSearchConfig) -> Self {
        Self {
            thread_count: match config.thread_count {
                MoveThreadCount::Auto => ThreadCount::Auto,
                MoveThreadCount::None => ThreadCount::Specific(NonZeroUsize::MIN),
                MoveThreadCount::Count(count) => ThreadCount::Specific(count),
            },
            log_progress: config.log_progress,
        }
    }
}
