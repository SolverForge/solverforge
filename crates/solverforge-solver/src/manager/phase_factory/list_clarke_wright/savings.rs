use std::collections::HashMap;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::owner_assignment::OwnerSlot;
use super::{ClarkeWrightAccess, SourceElement};
use crate::phase::construction::record_construction_candidate;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SavingsEntry {
    pub(crate) saving: i64,
    pub(crate) metric_class: usize,
    pub(crate) left_idx: usize,
    pub(crate) right_idx: usize,
}

pub(crate) fn sort_savings(entries: &mut [SavingsEntry]) {
    entries.sort_unstable_by(|left, right| {
        right
            .saving
            .cmp(&left.saving)
            .then_with(|| left.metric_class.cmp(&right.metric_class))
            .then_with(|| left.left_idx.cmp(&right.left_idx))
            .then_with(|| left.right_idx.cmp(&right.right_idx))
    });
}

/// Scores every pair of unassigned elements once per representative owner,
/// recording each pair as a construction candidate. Returns the savings and
/// whether configured termination interrupted the generation.
pub(super) fn generate_savings<S, A, D, BestCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    access: &A,
    unassigned: &[SourceElement<A::Element>],
    representative_owner_slots: &[OwnerSlot],
    trace_metric_class_by_value: &HashMap<usize, u64>,
    control_policy: StepControlPolicy,
) -> (Vec<SavingsEntry>, bool)
where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let n = unassigned.len();
    let mut savings = Vec::with_capacity(
        n.saturating_mul(n.saturating_sub(1))
            .saturating_div(2)
            .saturating_mul(representative_owner_slots.len()),
    );
    for owner_slot in representative_owner_slots {
        let owner_idx = owner_slot.owner_idx;
        for a in 0..n {
            if (a & 0x1F) == 0
                && control_policy.should_terminate_construction(phase_scope.solver_scope_mut())
            {
                return (savings, true);
            }
            for b in (a + 1)..n {
                let left = &unassigned[a];
                let right = &unassigned[b];
                let trace_token = phase_scope.record_candidate_operation(
                    CandidateTraceSource::ListClarkeWrightSavings,
                    None,
                    savings.len(),
                    Some(CandidateTraceConstructionTarget {
                        descriptor_index: access.descriptor_index(),
                        entity_index: owner_idx,
                    }),
                    access.descriptor_index(),
                    "clarke_wright_savings_pair",
                    [
                        owner_idx as u64,
                        trace_metric_class_by_value[&owner_slot.metric_class],
                        left.source_index as u64,
                        right.source_index as u64,
                    ],
                );
                let solution = phase_scope.score_director().working_solution();
                let depot = access.savings_depot(solution, owner_idx);
                let left_value = access.route_value(&left.element);
                let right_value = access.route_value(&right.element);
                let saving = super::sum_two_minus_one(
                    access.savings_distance(solution, owner_idx, depot, left_value),
                    access.savings_distance(solution, owner_idx, depot, right_value),
                    access.savings_distance(solution, owner_idx, left_value, right_value),
                );
                savings.push(SavingsEntry {
                    saving,
                    metric_class: owner_slot.metric_class,
                    left_idx: left.source_index,
                    right_idx: right.source_index,
                });
                if let Some(token) = trace_token {
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::Evaluated,
                    );
                    phase_scope.record_candidate_trace_disposition(
                        token,
                        CandidateTraceDisposition::ForagerIgnored,
                    );
                }
                record_construction_candidate(
                    phase_scope,
                    std::time::Duration::ZERO,
                    std::time::Duration::ZERO,
                );
                if (savings.len() & 0x3FF) == 0
                    && control_policy.should_terminate_construction(phase_scope.solver_scope_mut())
                {
                    return (savings, true);
                }
            }
        }
    }
    (savings, false)
}
