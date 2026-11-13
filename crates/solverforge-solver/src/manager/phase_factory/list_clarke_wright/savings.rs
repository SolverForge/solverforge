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

/// Scores pairs of unassigned elements once per representative owner,
/// recording each pair as a construction candidate: every pair when
/// `savings_neighbors` is `None`, otherwise only pairs where one element is
/// among the other's `savings_neighbors` nearest. Each pair is scored once,
/// left before right in `unassigned` order, so a bound of at least
/// `unassigned.len() - 1` yields exactly the unbounded savings. Returns the
/// savings and whether configured termination interrupted the generation.
pub(super) fn generate_savings<S, A, D, BestCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    access: &A,
    unassigned: &[SourceElement<A::Element>],
    representative_owner_slots: &[OwnerSlot],
    trace_metric_class_by_value: &HashMap<usize, u64>,
    savings_neighbors: Option<usize>,
    control_policy: StepControlPolicy,
) -> (Vec<SavingsEntry>, bool)
where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut generator = SavingsGenerator {
        access,
        unassigned,
        trace_metric_class_by_value,
        control_policy,
        savings: Vec::new(),
    };
    let interrupted = match savings_neighbors {
        None => generator.all_pairs(phase_scope, representative_owner_slots),
        Some(k) => generator.neighbor_pairs(phase_scope, representative_owner_slots, k),
    };
    (generator.savings, interrupted)
}

struct SavingsGenerator<'a, A, E> {
    access: &'a A,
    unassigned: &'a [SourceElement<E>],
    trace_metric_class_by_value: &'a HashMap<usize, u64>,
    control_policy: StepControlPolicy,
    savings: Vec<SavingsEntry>,
}

impl<A, E> SavingsGenerator<'_, A, E> {
    fn all_pairs<S, D, BestCb>(
        &mut self,
        phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
        representative_owner_slots: &[OwnerSlot],
    ) -> bool
    where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S, Element = E>,
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        let n = self.unassigned.len();
        self.savings.reserve(
            n.saturating_mul(n.saturating_sub(1))
                .saturating_div(2)
                .saturating_mul(representative_owner_slots.len()),
        );
        for owner_slot in representative_owner_slots {
            for a in 0..n {
                if (a & 0x1F) == 0
                    && self
                        .control_policy
                        .should_terminate_construction(phase_scope.solver_scope_mut())
                {
                    return true;
                }
                for b in (a + 1)..n {
                    if self.score_pair(phase_scope, owner_slot, a, b) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn neighbor_pairs<S, D, BestCb>(
        &mut self,
        phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
        representative_owner_slots: &[OwnerSlot],
        k: usize,
    ) -> bool
    where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S, Element = E>,
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        let n = self.unassigned.len();
        let stride = k.min(n.saturating_sub(1));
        let values: Vec<usize> = self
            .unassigned
            .iter()
            .map(|entry| self.access.route_value(&entry.element))
            .collect();
        let mut neighbors = Vec::with_capacity(n.saturating_mul(stride));
        self.savings.reserve(
            n.saturating_mul(stride)
                .saturating_mul(representative_owner_slots.len()),
        );
        for owner_slot in representative_owner_slots {
            neighbors.clear();
            self.access.savings_neighbors(
                phase_scope.score_director().working_solution(),
                owner_slot.owner_idx,
                &values,
                k,
                &mut neighbors,
            );
            assert_eq!(
                neighbors.len(),
                n * stride,
                "savings_neighbors_fn must append min(k, len - 1) = {stride} neighbours for each of {n} values"
            );
            for a in 0..n {
                if (a & 0x1F) == 0
                    && self
                        .control_policy
                        .should_terminate_construction(phase_scope.solver_scope_mut())
                {
                    return true;
                }
                for &b in &neighbors[a * stride..(a + 1) * stride] {
                    assert!(
                        b < n && b != a,
                        "savings_neighbors_fn returned neighbour index {b} for value index {a} of {n}"
                    );
                    // A pair both rows name is scored from the lower row.
                    if b < a && neighbors[b * stride..(b + 1) * stride].contains(&a) {
                        continue;
                    }
                    if self.score_pair(phase_scope, owner_slot, a.min(b), a.max(b)) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Scores the pair of unassigned positions `left < right` for one
    /// representative owner. Returns whether termination interrupted.
    fn score_pair<S, D, BestCb>(
        &mut self,
        phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
        owner_slot: &OwnerSlot,
        left: usize,
        right: usize,
    ) -> bool
    where
        S: PlanningSolution,
        A: ClarkeWrightAccess<S, Element = E>,
        D: Director<S>,
        BestCb: ProgressCallback<S>,
    {
        let access = self.access;
        let owner_idx = owner_slot.owner_idx;
        let left = &self.unassigned[left];
        let right = &self.unassigned[right];
        let trace_token = phase_scope.record_candidate_operation(
            CandidateTraceSource::ListClarkeWrightSavings,
            None,
            self.savings.len(),
            Some(CandidateTraceConstructionTarget {
                descriptor_index: access.descriptor_index(),
                entity_index: owner_idx,
            }),
            access.descriptor_index(),
            "clarke_wright_savings_pair",
            [
                owner_idx as u64,
                self.trace_metric_class_by_value[&owner_slot.metric_class],
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
        self.savings.push(SavingsEntry {
            saving,
            metric_class: owner_slot.metric_class,
            left_idx: left.source_index,
            right_idx: right.source_index,
        });
        if let Some(token) = trace_token {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
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
        (self.savings.len() & 0x3FF) == 0
            && self
                .control_policy
                .should_terminate_construction(phase_scope.solver_scope_mut())
    }
}
