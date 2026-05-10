use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{AssignmentMoveIntent, ScalarAssignmentMoveOptions};
use super::assignment_state::ScalarAssignmentState;
use super::move_build::compound_move_for_assignment_edits;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use crate::planning::ScalarEdit;

pub(super) fn assignment_move_for_entity_value<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    entity_index: usize,
    value: usize,
    options: ScalarAssignmentMoveOptions,
    intent: AssignmentMoveIntent,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    if state.current_value(entity_index) == Some(value) {
        return None;
    }
    let mut changes = Vec::new();
    let mut edits = Vec::new();
    let mut visiting = HashSet::new();
    let search = AugmentingPathSearch {
        group,
        solution,
        allow_optional_displacement: intent.allow_optional_displacement,
        value_candidate_limit: options.value_candidate_limit,
    };
    let produced = search.assign(
        state,
        AssignmentRequest {
            entity_index,
            value,
            depth: options.max_depth,
        },
        &mut visiting,
        &mut changes,
        &mut edits,
    );
    let mov = if produced && state.scalar_edits_feasible(group, solution, &edits) {
        move_from_edits(group, solution, &edits, intent.reason)
    } else {
        None
    };
    state.rollback(group, solution, &mut changes, 0);
    mov
}

#[derive(Clone, Copy)]
struct AssignmentRequest {
    entity_index: usize,
    value: usize,
    depth: usize,
}

struct AugmentingPathSearch<'a, S> {
    group: &'a ScalarAssignmentBinding<S>,
    solution: &'a S,
    allow_optional_displacement: bool,
    value_candidate_limit: Option<usize>,
}

impl<S> AugmentingPathSearch<'_, S> {
    fn assign(
        &self,
        state: &mut ScalarAssignmentState,
        assignment: AssignmentRequest,
        visiting: &mut HashSet<usize>,
        changes: &mut Vec<(usize, Option<usize>)>,
        edits: &mut Vec<ScalarEdit<S>>,
    ) -> bool {
        let entity_index = assignment.entity_index;
        let value = assignment.value;
        if !self
            .group
            .value_is_legal(self.solution, entity_index, Some(value))
        {
            return false;
        }
        if state.current_value(entity_index) == Some(value) {
            return true;
        }

        let mut blockers = state.blockers(self.group, self.solution, entity_index, value);
        if self.allow_optional_displacement {
            for blocker in blockers.iter().copied() {
                if state.is_required(blocker) {
                    continue;
                }
                state.set_value_recording(self.group, self.solution, blocker, None, changes);
                edits.push(self.group.edit(blocker, None));
            }
            blockers = state.blockers(self.group, self.solution, entity_index, value);
        }

        if blockers.is_empty()
            && !state.assignment_allowed(self.group, self.solution, entity_index, value)
        {
            return false;
        }

        if blockers.is_empty() {
            state.set_value_recording(
                self.group,
                self.solution,
                entity_index,
                Some(value),
                changes,
            );
            edits.push(self.group.edit(entity_index, Some(value)));
            return true;
        }

        if assignment.depth == 0 || !visiting.insert(entity_index) {
            return false;
        }

        let Some(blocker) = blockers
            .into_iter()
            .find(|blocker| state.is_required(*blocker))
        else {
            visiting.remove(&entity_index);
            return false;
        };

        let alternatives =
            self.group
                .candidate_values(self.solution, blocker, self.value_candidate_limit);
        for alternative in alternatives {
            if state.current_value(blocker) == Some(alternative) {
                continue;
            }
            let change_checkpoint = changes.len();
            let edit_checkpoint = edits.len();
            if self.assign(
                state,
                AssignmentRequest {
                    entity_index: blocker,
                    value: alternative,
                    depth: assignment.depth - 1,
                },
                visiting,
                changes,
                edits,
            ) {
                let blockers_after_reassignment =
                    state.blockers(self.group, self.solution, entity_index, value);
                if !blockers_after_reassignment.is_empty()
                    || !state.assignment_allowed(self.group, self.solution, entity_index, value)
                {
                    state.rollback(self.group, self.solution, changes, change_checkpoint);
                    edits.truncate(edit_checkpoint);
                    continue;
                }
                state.set_value_recording(
                    self.group,
                    self.solution,
                    entity_index,
                    Some(value),
                    changes,
                );
                edits.push(self.group.edit(entity_index, Some(value)));
                visiting.remove(&entity_index);
                return true;
            }
            state.rollback(self.group, self.solution, changes, change_checkpoint);
            edits.truncate(edit_checkpoint);
        }

        visiting.remove(&entity_index);
        false
    }
}

pub(super) fn move_from_edits<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    edits: &[ScalarEdit<S>],
    reason: &'static str,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    compound_move_for_assignment_edits(group, solution, edits, reason)
}
