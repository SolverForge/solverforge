use super::super::partitioner::ThreadCount;
use super::*;

#[test]
fn test_config_default() {
    let config = PartitionedSearchConfig::default();
    assert_eq!(config.thread_count, ThreadCount::Auto);
    assert!(!config.log_progress);
}
