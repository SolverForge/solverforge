use solverforge_core::domain::PlanningSolution;
use solverforge_core::fast_hash::FastSet;

use super::assignment_candidate::{
    order_candidates, AssignmentMoveIntent, ScalarAssignmentMoveOptions,
};
use super::assignment_entity::{
    required_entities_by_scarcity, required_value_degrees, sort_values_by_required_scarcity,
};
use super::assignment_path::{
    move_from_edits, AssignmentPathWork, AssignmentRequest, AugmentingPathSearch,
};
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

pub(super) fn required_batch_move<S, ShouldStop>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    options: ScalarAssignmentMoveOptions,
    should_stop: &mut ShouldStop,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
    ShouldStop: FnMut() -> bool,
{
    let mut entities = required_entities_by_scarcity(
        group,
        solution,
        state,
        options.value_candidate_limit,
        should_stop,
    )?;
    order_candidates(&mut entities, options);
    if should_stop() {
        return None;
    }
    let value_degrees = required_value_degrees(
        group,
        solution,
        &entities,
        options.value_candidate_limit,
        should_stop,
    )?;
    let mut scalar_edits = Vec::new();
    let mut edited_entities = FastSet::default();
    let mut visiting = FastSet::default();
    let mut blockers = Vec::new();
    let mut overlay = state.take_path_overlay();
    overlay.clear();
    let search =
        AugmentingPathSearch::new(group, solution, options, AssignmentMoveIntent::required());
    for entity_index in entities {
        if should_stop() {
            overlay.clear();
            state.restore_path_overlay(overlay);
            return None;
        }
        if overlay.current_value(state, entity_index).is_some() {
            continue;
        }
        let mut values =
            group.candidate_values(solution, entity_index, options.value_candidate_limit);
        sort_values_by_required_scarcity(
            group,
            solution,
            entity_index,
            &value_degrees,
            &mut values,
        );
        for value in values {
            if should_stop() {
                overlay.clear();
                state.restore_path_overlay(overlay);
                return None;
            }
            let overlay_checkpoint = overlay.checkpoint();
            let edit_checkpoint = scalar_edits.len();
            visiting.clear();
            let mut work = AssignmentPathWork::new(
                &mut overlay,
                &mut visiting,
                &mut scalar_edits,
                &mut blockers,
            );
            if !search.discover(
                state,
                // The batch below can retain only independent single-entity edits.
                // Defer augmenting rematches to the following required cursor,
                // where their multi-entity edits can actually be applied.
                AssignmentRequest::root(entity_index, value, 0),
                &mut work,
                should_stop,
            ) {
                overlay.rollback(overlay_checkpoint);
                scalar_edits.truncate(edit_checkpoint);
                if should_stop() {
                    overlay.clear();
                    state.restore_path_overlay(overlay);
                    return None;
                }
                continue;
            }
            let candidate_edits = &scalar_edits[edit_checkpoint..];
            if candidate_edits
                .iter()
                .any(|edit| edited_entities.contains(&edit.entity_index()))
            {
                overlay.rollback(overlay_checkpoint);
                scalar_edits.truncate(edit_checkpoint);
                continue;
            }
            if candidate_edits.len() != 1 {
                overlay.rollback(overlay_checkpoint);
                scalar_edits.truncate(edit_checkpoint);
                continue;
            }
            let candidate = &candidate_edits[0];
            if !group.value_is_legal(solution, candidate.entity_index(), candidate.to_value()) {
                overlay.rollback(overlay_checkpoint);
                scalar_edits.truncate(edit_checkpoint);
                continue;
            }
            for edit in candidate_edits {
                edited_entities.insert(edit.entity_index());
            }
            break;
        }
    }
    if should_stop() {
        overlay.clear();
        state.restore_path_overlay(overlay);
        return None;
    }
    overlay.clear();
    state.restore_path_overlay(overlay);
    if scalar_edits.is_empty() {
        return None;
    }
    if !state.scalar_edits_feasible(group, solution, &scalar_edits) {
        return None;
    }
    move_from_edits(group, solution, &scalar_edits, "scalar_assignment_required")
}
