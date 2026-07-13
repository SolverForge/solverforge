//! Canonical source-indexed round-robin list construction.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::context::SourceElement;
use crate::list_placement::OwnerRestriction;
use crate::scope::{PhaseScope, ProgressCallback, SolverScope, StepControlPolicy, StepScope};
use crate::stats::{
    CandidateTraceConstructionTarget, CandidateTraceDisposition, CandidateTraceSource,
};

/// The resolved operations used by canonical round-robin construction.
///
/// This is intentionally just an access protocol.  Static public phases and
/// compiled `RuntimeListSlot` instances supply storage-specific reads and
/// writes, while this kernel owns ordering, owner policy, candidate trace, and
/// mandatory-construction control flow exactly once.
pub(crate) trait RoundRobinAccess<S> {
    type Element: Clone + Send + Sync + 'static;

    fn descriptor_index(&self) -> usize;
    fn entity_count(&self, solution: &S) -> usize;
    fn construction_order_key(&self, solution: &S, element: &Self::Element) -> i64;
    fn owner_restriction(
        &self,
        solution: &S,
        entity_count: usize,
        element: &Self::Element,
    ) -> OwnerRestriction;
    fn append_element(&self, solution: &mut S, entity_index: usize, element: Self::Element);
}

/// Runs the one canonical round-robin construction algorithm over a frozen
/// source stream.
///
/// `all_assigned` is prepared outside the phase loop so public phase callers
/// preserve their established assigned-count handling and compiled callers can
/// supply the binder-validated equivalent.  Neither path rereads a declared
/// source while candidates are being applied.
pub(crate) fn run_round_robin<S, A, D, BestCb>(
    access: &A,
    source_count: usize,
    all_assigned: bool,
    bound_unassigned: &[SourceElement<A::Element>],
    control_policy: StepControlPolicy,
    solver_scope: &mut SolverScope<'_, S, D, BestCb>,
) where
    S: PlanningSolution,
    A: RoundRobinAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut phase_scope =
        PhaseScope::with_phase_type(solver_scope, 0, "Round-Robin List Construction");
    let n_entities = access.entity_count(phase_scope.score_director().working_solution());

    if n_entities == 0 || source_count == 0 {
        phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    if all_assigned {
        tracing::info!("All elements already assigned, skipping construction");
        phase_scope.score_director_mut().calculate_score();
        phase_scope.update_best_solution();
        return;
    }

    let mut elements = bound_unassigned.to_vec();
    let solution = phase_scope.score_director().working_solution();
    elements.sort_by_key(|entry| {
        (
            access.construction_order_key(solution, &entry.element),
            entry.source_index,
        )
    });

    let mut entity_idx = 0;
    for entry in elements {
        if control_policy.should_terminate_construction(phase_scope.solver_scope_mut()) {
            break;
        }

        let solution = phase_scope.score_director().working_solution();
        let (target_entity, advance_round_robin) =
            match access.owner_restriction(solution, n_entities, &entry.element) {
                OwnerRestriction::Unrestricted => (entity_idx, true),
                OwnerRestriction::Fixed(owner_idx) => (owner_idx, false),
                OwnerRestriction::Invalid => {
                    tracing::warn!("No valid owner found for list element");
                    continue;
                }
            };

        let descriptor_index = access.descriptor_index();
        let trace_token = phase_scope.record_candidate_operation(
            CandidateTraceSource::ListRoundRobinConstruction,
            None,
            entry.source_index,
            Some(CandidateTraceConstructionTarget {
                descriptor_index,
                entity_index: target_entity,
            }),
            descriptor_index,
            "round_robin_list_assignment",
            [entry.source_index as u64, target_entity as u64],
        );
        if let Some(token) = trace_token {
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Evaluated);
            phase_scope
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Selected);
        }

        let mut step_scope = StepScope::new_with_control_policy(&mut phase_scope, control_policy);
        step_scope.apply_committed_change(|sd| {
            sd.before_variable_changed(descriptor_index, target_entity);
            access.append_element(sd.working_solution_mut(), target_entity, entry.element);
            sd.after_variable_changed(descriptor_index, target_entity);
        });
        if let Some(token) = trace_token {
            step_scope
                .phase_scope_mut()
                .record_candidate_trace_disposition(token, CandidateTraceDisposition::Applied);
        }

        let step_score = step_scope.calculate_score();
        step_scope.set_step_score(step_score);
        step_scope.complete();

        if advance_round_robin {
            entity_idx = (entity_idx + 1) % n_entities;
        }
    }

    phase_scope.update_best_solution();
}
