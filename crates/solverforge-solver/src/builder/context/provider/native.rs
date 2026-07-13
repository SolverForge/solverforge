//! Direct adapters for typed Rust compound providers.
//!
//! Native providers stay on their function-pointer and typed-edit path. Host
//! callback payloads use the separate raw-name protocol in the registry.

use crate::{RepairCandidate, RepairLimits, ScalarCandidate, ScalarGroupLimits};

use super::{StaticConflictRepairProviderBinding, StaticScalarGroupProviderBinding};

pub(super) fn pull_static_group<S>(
    binding: &StaticScalarGroupProviderBinding<S>,
    solution: &S,
    value_candidate_limit: Option<usize>,
    max_moves_per_step: Option<usize>,
) -> Vec<ScalarCandidate<S>> {
    let declared = binding.declared_limits;
    let limits = ScalarGroupLimits {
        value_candidate_limit: value_candidate_limit.or(declared.value_candidate_limit),
        group_candidate_limit: max_moves_per_step.or(declared.group_candidate_limit),
        max_moves_per_step: max_moves_per_step.or(declared.max_moves_per_step),
        max_augmenting_depth: declared.max_augmenting_depth,
        max_rematch_size: declared.max_rematch_size,
    };
    (binding.provider)(solution, limits)
}

pub(super) fn pull_static_repair<S>(
    binding: &StaticConflictRepairProviderBinding<S>,
    solution: &S,
    limits: RepairLimits,
) -> Vec<RepairCandidate<S>> {
    (binding.repair.provider())(solution, limits)
}
