use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{AssignmentMoveIntent, ScalarAssignmentMoveOptions};
use super::assignment_path_overlay::AssignmentPathOverlay;
use super::assignment_state::ScalarAssignmentState;
use super::move_build::compound_move_for_assignment_edits;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use crate::planning::ScalarEdit;
use solverforge_core::fast_hash::FastSet;

pub(super) fn assignment_move_for_entity_value<S, ShouldStop>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    assignment: AssignmentRequest,
    options: ScalarAssignmentMoveOptions,
    intent: AssignmentMoveIntent,
    should_stop: &mut ShouldStop,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
    ShouldStop: FnMut() -> bool,
{
    if should_stop() {
        return None;
    }
    let entity_index = assignment.entity_index;
    let value = assignment.value;
    if state.current_value(entity_index) == Some(value) {
        return None;
    }
    let mut overlay = state.take_path_overlay();
    overlay.clear();
    let mut edits = Vec::new();
    let mut visiting = FastSet::default();
    let mut blockers = Vec::new();
    let search = AugmentingPathSearch::new(group, solution, options, intent);
    let mut work = AssignmentPathWork::new(&mut overlay, &mut visiting, &mut edits, &mut blockers);
    let produced = search.discover(state, assignment, &mut work, should_stop);
    let feasible = produced && state.refresh_trial_edits(group, solution, &edits);
    overlay.clear();
    state.restore_path_overlay(overlay);
    feasible
        .then_some(move_from_edits(group, solution, &edits, intent.reason))
        .flatten()
}

#[derive(Clone, Copy)]
pub(super) struct AssignmentRequest {
    entity_index: usize,
    value: usize,
    depth: usize,
}

impl AssignmentRequest {
    pub(super) fn root(entity_index: usize, value: usize, max_depth: usize) -> Self {
        Self {
            entity_index,
            value,
            depth: max_depth,
        }
    }
}

pub(super) struct AugmentingPathSearch<'a, S> {
    group: &'a ScalarAssignmentBinding<S>,
    solution: &'a S,
    allow_optional_displacement: bool,
    value_candidate_limit: Option<usize>,
}

pub(super) struct AssignmentPathWork<'a, S> {
    overlay: &'a mut AssignmentPathOverlay,
    visiting: &'a mut FastSet<usize>,
    edits: &'a mut Vec<ScalarEdit<S>>,
    blockers: &'a mut Vec<usize>,
}

impl<'a, S> AssignmentPathWork<'a, S> {
    pub(super) fn new(
        overlay: &'a mut AssignmentPathOverlay,
        visiting: &'a mut FastSet<usize>,
        edits: &'a mut Vec<ScalarEdit<S>>,
        blockers: &'a mut Vec<usize>,
    ) -> Self {
        Self {
            overlay,
            visiting,
            edits,
            blockers,
        }
    }
}

impl<S> AugmentingPathSearch<'_, S> {
    pub(super) fn new<'a>(
        group: &'a ScalarAssignmentBinding<S>,
        solution: &'a S,
        options: ScalarAssignmentMoveOptions,
        intent: AssignmentMoveIntent,
    ) -> AugmentingPathSearch<'a, S> {
        AugmentingPathSearch {
            group,
            solution,
            allow_optional_displacement: intent.allow_optional_displacement,
            value_candidate_limit: options.value_candidate_limit,
        }
    }

    pub(super) fn discover<ShouldStop>(
        &self,
        state: &ScalarAssignmentState,
        assignment: AssignmentRequest,
        work: &mut AssignmentPathWork<'_, S>,
        should_stop: &mut ShouldStop,
    ) -> bool
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop()
            || work.overlay.current_value(state, assignment.entity_index) == Some(assignment.value)
        {
            return false;
        }
        self.assign(state, assignment, work, should_stop)
    }

    fn assign<ShouldStop>(
        &self,
        state: &ScalarAssignmentState,
        assignment: AssignmentRequest,
        work: &mut AssignmentPathWork<'_, S>,
        should_stop: &mut ShouldStop,
    ) -> bool
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            return false;
        }
        let entity_index = assignment.entity_index;
        let value = assignment.value;
        if !self
            .group
            .value_is_legal(self.solution, entity_index, Some(value))
        {
            return false;
        }
        if work.overlay.current_value(state, entity_index) == Some(value) {
            return true;
        }

        let mut rule_verdict = work.overlay.blockers(
            state,
            self.group,
            self.solution,
            entity_index,
            value,
            work.blockers,
        );
        if self.allow_optional_displacement {
            for blocker in work.blockers.iter().copied() {
                if state.is_required(blocker) {
                    continue;
                }
                work.overlay.set_value(state, blocker, None);
                work.edits.push(self.group.edit(blocker, None));
            }
            // State changed after the displacements; re-derive both blockers
            // and any rule verdict on the new state.
            rule_verdict = work.overlay.blockers(
                state,
                self.group,
                self.solution,
                entity_index,
                value,
                work.blockers,
            );
        }

        if work.blockers.is_empty()
            && !match rule_verdict {
                Some(allowed) => allowed,
                None => work.overlay.assignment_allowed(
                    state,
                    self.group,
                    self.solution,
                    entity_index,
                    value,
                ),
            }
        {
            return false;
        }

        if work.blockers.is_empty() {
            work.overlay.set_value(state, entity_index, Some(value));
            work.edits.push(self.group.edit(entity_index, Some(value)));
            return true;
        }

        if assignment.depth == 0 || !work.visiting.insert(entity_index) {
            return false;
        }

        let Some(blocker) = work
            .blockers
            .iter()
            .copied()
            .find(|blocker| state.is_required(*blocker))
        else {
            work.visiting.remove(&entity_index);
            return false;
        };

        let alternatives =
            self.group
                .candidate_values(self.solution, blocker, self.value_candidate_limit);
        for alternative in alternatives {
            if should_stop() {
                work.visiting.remove(&entity_index);
                return false;
            }
            if work.overlay.current_value(state, blocker) == Some(alternative) {
                continue;
            }
            let overlay_checkpoint = work.overlay.checkpoint();
            let edit_checkpoint = work.edits.len();
            if self.assign(
                state,
                AssignmentRequest {
                    entity_index: blocker,
                    value: alternative,
                    depth: assignment.depth - 1,
                },
                work,
                should_stop,
            ) {
                let reassign_verdict = work.overlay.blockers(
                    state,
                    self.group,
                    self.solution,
                    entity_index,
                    value,
                    work.blockers,
                );
                if !work.blockers.is_empty()
                    || !match reassign_verdict {
                        Some(allowed) => allowed,
                        None => work.overlay.assignment_allowed(
                            state,
                            self.group,
                            self.solution,
                            entity_index,
                            value,
                        ),
                    }
                {
                    work.overlay.rollback(overlay_checkpoint);
                    work.edits.truncate(edit_checkpoint);
                    continue;
                }
                work.overlay.set_value(state, entity_index, Some(value));
                work.edits.push(self.group.edit(entity_index, Some(value)));
                work.visiting.remove(&entity_index);
                return true;
            }
            work.overlay.rollback(overlay_checkpoint);
            work.edits.truncate(edit_checkpoint);
        }

        work.visiting.remove(&entity_index);
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
