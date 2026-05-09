use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::assignment_candidate::{
    ordered_entities, remaining_required_count, AssignmentMoveIntent, ScalarAssignmentMoveOptions,
};
use super::assignment_path::assignment_moves_for_entity;
use super::assignment_state::ScalarAssignmentState;
use super::candidate::{CandidateAcceptance, NormalizedGroupedCandidate};
use crate::builder::ScalarAssignmentBinding;
use crate::descriptor::ResolvedVariableBinding;
use crate::heuristic::r#move::{CompoundScalarEdit, Move};
use crate::phase::construction::{ConstructionGroupSlotId, ConstructionGroupSlotKey};
use crate::scope::{PhaseScope, ProgressCallback};

pub(super) fn normalize_assignment_candidates<S, D, ProgressCb>(
    phase_scope: &PhaseScope<'_, '_, S, D, ProgressCb>,
    group_index: usize,
    assignment: ScalarAssignmentBinding<S>,
    scalar_bindings: &[ResolvedVariableBinding<S>],
    options: ScalarAssignmentMoveOptions,
) -> Vec<NormalizedGroupedCandidate<S>>
where
    S: PlanningSolution + 'static,
    D: Director<S>,
    ProgressCb: ProgressCallback<S>,
{
    let solution = phase_scope.score_director().working_solution();
    let Some(target_binding) = assignment_target_binding(&assignment, scalar_bindings) else {
        panic!(
            "assignment-backed grouped scalar construction targets unknown scalar slot {}.{}",
            assignment.target.entity_type_name, assignment.target.variable_name
        );
    };
    if options.max_moves == 0 {
        return Vec::new();
    }

    let state = ScalarAssignmentState::new(&assignment, solution);
    let construction_mode = if remaining_required_count(&assignment, solution) > 0 {
        AssignmentConstructionMode::Required
    } else {
        AssignmentConstructionMode::Optional
    };
    let entities = ordered_entities(&assignment, solution, |entity_index| {
        let unassigned = state.current_value(entity_index).is_none();
        match construction_mode {
            AssignmentConstructionMode::Required => state.is_required(entity_index) && unassigned,
            AssignmentConstructionMode::Optional => !state.is_required(entity_index) && unassigned,
        }
    });

    let mut normalized = Vec::new();
    let mut seen = Vec::new();
    let mut sequence = 0;
    for entity_index in entities {
        if normalized.len() >= options.max_moves {
            break;
        }
        let slot_id = target_binding.slot_id(entity_index);
        if phase_scope.solver_scope().is_scalar_slot_completed(slot_id) {
            continue;
        }
        let group_slot = assignment_group_slot(group_index, entity_index);
        if phase_scope
            .solver_scope()
            .is_group_slot_completed(&group_slot)
        {
            continue;
        }

        let moves = assignment_moves_for_entity(
            &assignment,
            solution,
            entity_index,
            options,
            construction_mode.intent(),
        );
        for mov in moves {
            if normalized.len() >= options.max_moves {
                break;
            }
            if !mov.is_doable(phase_scope.score_director()) {
                continue;
            }
            let Some(principal) = principal_assignment_edit(&mov, entity_index) else {
                continue;
            };
            let signature = move_signature(&mov);
            if seen
                .iter()
                .any(|seen_signature| seen_signature == &signature)
            {
                continue;
            }
            let value_order_key = principal
                .to_value
                .and_then(|value| assignment.value_order_key(solution, entity_index, value));
            normalized.push(NormalizedGroupedCandidate {
                sequence,
                group_slot: assignment_group_slot(group_index, entity_index),
                scalar_slots: vec![slot_id],
                keep_current_legal: assignment.target.allows_unassigned,
                entity_order_key: assignment.entity_order_key(solution, entity_index),
                value_order_key,
                acceptance: construction_mode.acceptance(),
                mov,
            });
            seen.push(signature);
            sequence += 1;
        }
    }

    normalized
}

#[derive(Clone, Copy)]
enum AssignmentConstructionMode {
    Required,
    Optional,
}

impl AssignmentConstructionMode {
    fn intent(self) -> AssignmentMoveIntent {
        match self {
            Self::Required => AssignmentMoveIntent::required(),
            Self::Optional => AssignmentMoveIntent::optional(),
        }
    }

    fn acceptance(self) -> CandidateAcceptance {
        match self {
            Self::Required => CandidateAcceptance::RequiredAssignment,
            Self::Optional => CandidateAcceptance::OptionalAssignment,
        }
    }
}

fn assignment_target_binding<'a, S>(
    assignment: &ScalarAssignmentBinding<S>,
    scalar_bindings: &'a [ResolvedVariableBinding<S>],
) -> Option<&'a ResolvedVariableBinding<S>> {
    scalar_bindings.iter().find(|binding| {
        binding.descriptor_index == assignment.target.descriptor_index
            && binding.variable_index == assignment.target.variable_index
    })
}

fn assignment_group_slot(group_index: usize, entity_index: usize) -> ConstructionGroupSlotId {
    ConstructionGroupSlotId::new(
        group_index,
        ConstructionGroupSlotKey::Explicit(entity_index),
    )
}

fn principal_assignment_edit<S>(
    mov: &crate::heuristic::r#move::CompoundScalarMove<S>,
    entity_index: usize,
) -> Option<&CompoundScalarEdit<S>> {
    mov.edits()
        .iter()
        .find(|edit| edit.entity_index == entity_index && edit.to_value.is_some())
}

fn move_signature<S>(
    mov: &crate::heuristic::r#move::CompoundScalarMove<S>,
) -> Vec<(usize, Option<usize>)> {
    let mut signature = mov
        .edits()
        .iter()
        .map(|edit| (edit.entity_index, edit.to_value))
        .collect::<Vec<_>>();
    signature.sort_unstable();
    signature
}
