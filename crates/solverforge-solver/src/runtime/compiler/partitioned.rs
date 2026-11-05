//! The configuration a `partitioned_search` phase solves each partition
//! with.

use solverforge_config::{PartitionedSearchConfig, SolverConfig, TerminationConfig};

/// The solver configuration of one partition's solve: the parent's, with
/// `child_phases` as its phases (empty means the default phases, as at the
/// top level) and as termination the bounds that reach the partition.
///
/// A partition runs under the parent's time limit and step count budget;
/// limits that need the parent's best score are not checked inside it. The
/// child's termination therefore carries only the time and step count
/// limits, so the default local search stays construction-only in a
/// partition exactly when nothing bounds it there.
pub(crate) fn partition_child_config(
    parent: &SolverConfig,
    partitioned: &PartitionedSearchConfig,
) -> SolverConfig {
    let mut child = parent.clone();
    child.phases = partitioned.child_phases.clone();
    child.termination = partition_bounds(parent.termination.as_ref());
    child
}

fn partition_bounds(parent: Option<&TerminationConfig>) -> Option<TerminationConfig> {
    let parent = parent?;
    let bounds = TerminationConfig {
        seconds_spent_limit: parent.time_limit().map(|limit| limit.as_secs()),
        step_count_limit: parent.step_count_limit,
        ..TerminationConfig::default()
    };
    (bounds.seconds_spent_limit.is_some() || bounds.step_count_limit.is_some()).then_some(bounds)
}
