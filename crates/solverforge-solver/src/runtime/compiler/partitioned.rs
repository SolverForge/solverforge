//! The configuration a `partitioned_search` phase solves each partition
//! with.

use std::time::Duration;

use solverforge_config::{PartitionedSearchConfig, SolverConfig, TerminationConfig};

/// The solver configuration of one partition's solve: the parent's, with
/// `child_phases` as its phases (empty means the default phases, as at the
/// top level) and as termination the bounds that reach the partition.
///
/// A partition runs under the solve's time limit and step count budget and
/// under the partitioned phase's own time and step count limits; limits
/// that need the parent's best score are not checked inside it. The child's
/// termination therefore carries the earliest time and step count limits,
/// so the default local search stays construction-only in a partition
/// exactly when nothing bounds it there.
pub(crate) fn partition_child_config(
    parent: &SolverConfig,
    partitioned: &PartitionedSearchConfig,
) -> SolverConfig {
    let mut child = parent.clone();
    child.phases = partitioned.child_phases.clone();
    child.termination = partition_bounds(
        parent.termination.as_ref(),
        partitioned.termination.as_ref(),
    );
    child
}

fn partition_bounds(
    solve: Option<&TerminationConfig>,
    phase: Option<&TerminationConfig>,
) -> Option<TerminationConfig> {
    let time_limit = earliest(
        solve.and_then(TerminationConfig::time_limit),
        phase.and_then(TerminationConfig::time_limit),
    );
    let step_count_limit = earliest(
        solve.and_then(|config| config.step_count_limit),
        phase.and_then(|config| config.step_count_limit),
    );
    (time_limit.is_some() || step_count_limit.is_some()).then(|| TerminationConfig {
        seconds_spent_limit: time_limit.map(|limit: Duration| limit.as_secs()),
        step_count_limit,
        ..TerminationConfig::default()
    })
}

fn earliest<T: Ord>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (left, right) => left.or(right),
    }
}
