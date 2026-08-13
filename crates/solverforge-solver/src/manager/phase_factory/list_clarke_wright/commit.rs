// Atomic route-assignment commits for Clarke-Wright construction.

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::completion::{complete_routes_by_insertion, CompletionSelection};
use super::owner_assignment::OwnerSlot;
use super::route_state::ConstructedRoute;
use super::{ClarkeWrightAccess, RuntimeListSourceIndex};
use crate::phase::construction::PendingConstructionMoveTelemetry;
use crate::scope::{PhaseScope, ProgressCallback, StepControlPolicy, StepScope};

#[allow(clippy::too_many_arguments)]
pub(super) fn try_commit_balanced_assignment<S, A, D, BestCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    access: &A,
    source_index: &RuntimeListSourceIndex<A::Element>,
    owner_slots: &[OwnerSlot],
    routes: &[ConstructedRoute],
    entity_count: usize,
    available_entity_slots: &[usize],
    control_policy: StepControlPolicy,
) where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let mut telemetry = PendingConstructionMoveTelemetry::default();
    let Some(completed_routes) = complete_routes_by_insertion(
        phase_scope,
        access,
        source_index,
        owner_slots,
        routes,
        entity_count,
        CompletionSelection::Balanced,
        control_policy,
        &mut telemetry,
    ) else {
        telemetry.record_discarded(phase_scope);
        return;
    };
    commit_complete_routes(
        phase_scope,
        access,
        available_entity_slots,
        completed_routes,
        control_policy,
        telemetry,
    );
    phase_scope.update_best_solution();
    phase_scope.promote_current_solution_on_score_tie();
}

pub(super) fn commit_complete_routes<S, A, D, BestCb>(
    phase_scope: &mut PhaseScope<'_, '_, S, D, BestCb>,
    access: &A,
    available_entity_slots: &[usize],
    completed_routes: Vec<(usize, Vec<usize>)>,
    control_policy: StepControlPolicy,
    pending_move_telemetry: PendingConstructionMoveTelemetry,
) where
    S: PlanningSolution,
    A: ClarkeWrightAccess<S>,
    D: Director<S>,
    BestCb: ProgressCallback<S>,
{
    let entity_count = access.entity_count(phase_scope.score_director().working_solution());
    let mut route_by_owner = (0..entity_count).map(|_| None).collect::<Vec<_>>();
    for (owner_idx, route) in completed_routes {
        assert!(
            owner_idx < entity_count && route_by_owner[owner_idx].is_none(),
            "Clarke-Wright completion produced an invalid or duplicate route owner"
        );
        route_by_owner[owner_idx] = Some(route);
    }

    let descriptor_index = access.descriptor_index();
    let mut step_scope = StepScope::new_with_control_policy(phase_scope, control_policy);
    step_scope.apply_committed_change(|director| {
        for &entity_idx in available_entity_slots {
            director.before_variable_changed(descriptor_index, entity_idx);
            let route = route_by_owner[entity_idx].take().unwrap_or_default();
            access.replace_route(director.working_solution_mut(), entity_idx, route);
            director.after_variable_changed(descriptor_index, entity_idx);
        }
    });
    pending_move_telemetry.record_committed(step_scope.phase_scope_mut());
    let step_score = step_scope.calculate_score();
    step_scope.set_step_score(step_score);
    step_scope.complete();
}
