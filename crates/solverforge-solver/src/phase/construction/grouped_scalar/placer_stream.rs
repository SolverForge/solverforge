use solverforge_config::{ConstructionHeuristicType, ConstructionObligation};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::assignment_candidate::ScalarAssignmentMoveOptions;
use super::assignment_stream::ScalarAssignmentMoveCursor;
use super::placement::{
    assignment_group_slot, assignment_move_target, assignment_move_touches_completed_slot,
    placement_entity_order_key, placement_sequence, principal_assignment_edit,
};
use super::placer::ScalarGroupPlacement;
use crate::builder::context::ScalarAssignmentBinding;
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarMove, Move};
use crate::heuristic::selector::EntityReference;
use crate::phase::construction::capabilities::grouped_heuristic_requires_entity_order;
use crate::phase::construction::Placement;

pub(super) struct CandidatePlacementGenerator<S>
where
    S: PlanningSolution + 'static,
{
    pub(super) placements: std::vec::IntoIter<ScalarGroupPlacement<S>>,
}

pub(super) struct AssignmentPlacementGenerator<S>
where
    S: PlanningSolution + 'static,
{
    pub(super) group_index: usize,
    pub(super) assignment: ScalarAssignmentBinding<S>,
    pub(super) target_binding: ResolvedVariableBinding<S>,
    pub(super) cursor: ScalarAssignmentMoveCursor<S>,
    pub(super) pending: Option<CompoundScalarMove<S>>,
    pub(super) options: ScalarAssignmentMoveOptions,
    pub(super) accepted: usize,
}

pub(super) fn next_candidate_placement<S, IsCompleted>(
    generator: &mut CandidatePlacementGenerator<S>,
    generated_moves: &mut u64,
    is_completed: &mut IsCompleted,
) -> Option<ScalarGroupPlacement<S>>
where
    S: PlanningSolution + 'static,
    IsCompleted: FnMut(&ScalarGroupPlacement<S>) -> bool,
{
    for placement in generator.placements.by_ref() {
        *generated_moves = generated_moves
            .saturating_add(u64::try_from(placement.moves.len()).unwrap_or(u64::MAX));
        if is_completed(&placement) {
            continue;
        }
        return Some(placement);
    }
    None
}

pub(super) fn next_assignment_placement<S, D, IsCompleted>(
    generator: &mut AssignmentPlacementGenerator<S>,
    score_director: &D,
    generated_moves: &mut u64,
    is_completed: &mut IsCompleted,
) -> Option<ScalarGroupPlacement<S>>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    IsCompleted: FnMut(&ScalarGroupPlacement<S>) -> bool,
{
    let solution = score_director.working_solution();
    while generator.accepted < generator.options.max_moves {
        let (entity_index, mov) =
            next_assignment_move_for_placement(generator, score_director, is_completed)?;
        *generated_moves = generated_moves.saturating_add(1);
        let group_slot = assignment_group_slot(generator.group_index, entity_index);
        let mut moves = vec![assignment_move_with_order_key(
            &generator.assignment,
            solution,
            entity_index,
            mov,
        )];
        while generator.accepted + moves.len() < generator.options.max_moves {
            let Some((next_entity, next_move)) =
                next_assignment_move_for_placement(generator, score_director, is_completed)
            else {
                break;
            };
            if next_entity != entity_index {
                generator.pending = Some(next_move);
                break;
            }
            *generated_moves = generated_moves.saturating_add(1);
            moves.push(assignment_move_with_order_key(
                &generator.assignment,
                solution,
                entity_index,
                next_move,
            ));
        }

        let move_targets = moves
            .iter()
            .map(|mov| assignment_move_target(&generator.target_binding, &group_slot, mov))
            .collect::<Vec<_>>();
        let mut scalar_slots = moves
            .iter()
            .flat_map(|mov| {
                mov.edits()
                    .iter()
                    .map(|edit| generator.target_binding.slot_id(edit.entity_index))
            })
            .collect::<Vec<_>>();
        scalar_slots.sort_unstable();
        scalar_slots.dedup();
        let placement = Placement::new(
            EntityReference::new(generator.assignment.target.descriptor_index, entity_index),
            moves,
        )
        .with_group_slot(group_slot)
        .with_scalar_slots(scalar_slots)
        .with_move_targets(move_targets)
        .with_keep_current_legal(generator.assignment.target.allows_unassigned)
        .with_construction_entity_order_key(
            generator
                .assignment
                .entity_order_key(solution, entity_index),
        );
        if is_completed(&placement) {
            continue;
        }
        generator.accepted += placement.moves.len();
        return Some(placement);
    }
    None
}

fn next_assignment_move_for_placement<S, D, IsCompleted>(
    generator: &mut AssignmentPlacementGenerator<S>,
    score_director: &D,
    is_completed: &mut IsCompleted,
) -> Option<(usize, CompoundScalarMove<S>)>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    IsCompleted: FnMut(&ScalarGroupPlacement<S>) -> bool,
{
    loop {
        let mov = if let Some(mov) = generator.pending.take() {
            mov
        } else {
            generator.cursor.next_move()?
        };
        if assignment_move_touches_completed_slot(
            &mov,
            &generator.target_binding,
            generator.assignment.target.descriptor_index,
            is_completed,
        ) {
            continue;
        }
        if !mov.is_doable(score_director) {
            continue;
        }
        let Some(anchor) = mov
            .edits()
            .iter()
            .find(|edit| edit.to_value.is_some())
            .or_else(|| mov.edits().first())
        else {
            continue;
        };
        return Some((anchor.entity_index, mov));
    }
}

fn assignment_move_with_order_key<S>(
    assignment: &ScalarAssignmentBinding<S>,
    solution: &S,
    entity_index: usize,
    mov: CompoundScalarMove<S>,
) -> CompoundScalarMove<S>
where
    S: PlanningSolution,
{
    let order_key = principal_assignment_edit(&mov, entity_index)
        .and_then(|principal| principal.to_value)
        .and_then(|value| assignment.value_order_key(solution, entity_index, value));
    mov.with_construction_value_order_key(order_key)
}

pub(super) fn sort_grouped_placements<S>(
    placements: &mut [ScalarGroupPlacement<S>],
    heuristic: ConstructionHeuristicType,
) where
    S: PlanningSolution + 'static,
{
    if grouped_heuristic_requires_entity_order(heuristic) {
        placements.sort_by(|left, right| {
            placement_entity_order_key(right)
                .cmp(&placement_entity_order_key(left))
                .then_with(|| placement_sequence(left).cmp(&placement_sequence(right)))
        });
    }
}

pub(super) fn assignment_placement_move_limit(
    heuristic: ConstructionHeuristicType,
    construction_obligation: ConstructionObligation,
    required_only: bool,
    entity_count: usize,
    options: ScalarAssignmentMoveOptions,
) -> usize {
    if matches!(
        heuristic,
        ConstructionHeuristicType::FirstFit | ConstructionHeuristicType::FirstFitDecreasing
    ) && matches!(
        construction_obligation,
        ConstructionObligation::AssignWhenCandidateExists
    ) {
        if required_only && matches!(heuristic, ConstructionHeuristicType::FirstFit) {
            return options.max_moves.min(1);
        }
        if options.max_moves != usize::MAX {
            return options.max_moves;
        }
        return entity_count
            .saturating_mul(options.max_rematch_size)
            .clamp(256, 4096);
    }
    options.max_moves
}
