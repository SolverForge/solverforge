use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{
    ordered_entities, AssignmentMoveIntent, ScalarAssignmentMoveOptions,
};
use super::assignment_cycle::CycleWindowCursor;
use super::assignment_entity::{
    required_entities_by_scarcity, required_value_degrees, sort_values_by_required_scarcity,
    AssignmentMoveKind, CapacityCursor, OptionalAdjustmentCursor,
};
use super::assignment_family::{AssignmentFamilyCursor, AssignmentMoveFamily};
use super::assignment_pair::PairWindowCursor;
use super::assignment_path::{assignment_move_for_entity_value, move_from_edits};
use super::assignment_state::ScalarAssignmentState;
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;

type AssignmentMoveKey = Vec<(usize, usize, usize, &'static str, Option<usize>)>;

pub(crate) struct ScalarAssignmentMoveCursor<S>
where
    S: PlanningSolution,
{
    group: ScalarAssignmentBinding<S>,
    solution: S,
    options: ScalarAssignmentMoveOptions,
    family: AssignmentMoveFamily,
    stop_after: AssignmentMoveFamily,
    spread_budget: bool,
    batch_required: bool,
    state: ScalarAssignmentState,
    active: AssignmentFamilyCursor<S>,
    active_limit: usize,
    active_yielded: usize,
    seen: HashSet<AssignmentMoveKey>,
    yielded: usize,
}

impl<S> ScalarAssignmentMoveCursor<S>
where
    S: PlanningSolution,
{
    pub(crate) fn new(
        group: ScalarAssignmentBinding<S>,
        solution: S,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self::from_family_range(
            group,
            solution,
            options,
            AssignmentMoveFamily::Required,
            AssignmentMoveFamily::EjectionReinsert,
            true,
            false,
        )
    }

    pub(crate) fn required_construction(
        group: ScalarAssignmentBinding<S>,
        solution: S,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self::from_family_range(
            group,
            solution,
            options,
            AssignmentMoveFamily::Required,
            AssignmentMoveFamily::Required,
            false,
            true,
        )
    }

    pub(crate) fn required(
        group: ScalarAssignmentBinding<S>,
        solution: S,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self::from_family_range(
            group,
            solution,
            options,
            AssignmentMoveFamily::Required,
            AssignmentMoveFamily::Required,
            false,
            false,
        )
    }

    pub(crate) fn optional_construction(
        group: ScalarAssignmentBinding<S>,
        solution: S,
        options: ScalarAssignmentMoveOptions,
    ) -> Self {
        Self::from_family_range(
            group,
            solution,
            options,
            AssignmentMoveFamily::OptionalAssign,
            AssignmentMoveFamily::OptionalAssign,
            false,
            false,
        )
    }

    fn from_family_range(
        group: ScalarAssignmentBinding<S>,
        solution: S,
        options: ScalarAssignmentMoveOptions,
        family: AssignmentMoveFamily,
        stop_after: AssignmentMoveFamily,
        spread_budget: bool,
        batch_required: bool,
    ) -> Self {
        Self {
            state: ScalarAssignmentState::new(&group, &solution),
            group,
            solution,
            options,
            family,
            stop_after,
            spread_budget,
            batch_required,
            active: AssignmentFamilyCursor::Empty,
            active_limit: 0,
            active_yielded: 0,
            seen: HashSet::new(),
            yielded: 0,
        }
    }

    pub(crate) fn next_move(&mut self) -> Option<CompoundScalarMove<S>> {
        if self.options.max_moves == 0 || self.yielded >= self.options.max_moves {
            return None;
        }

        loop {
            if self.active_yielded >= self.active_limit {
                self.active = AssignmentFamilyCursor::Empty;
            }
            if let Some(candidate) = self.next_family_move() {
                if !self.seen.insert(normalized_move_key(&candidate)) {
                    continue;
                }
                self.active_yielded += 1;
                self.yielded += 1;
                return Some(candidate);
            }
            if !self.load_next_family() {
                return None;
            }
        }
    }

    fn load_next_family(&mut self) -> bool {
        while let Some(family) = self.family.take_next() {
            let remaining = self.options.max_moves.saturating_sub(self.yielded);
            if remaining == 0 {
                return false;
            }
            if family == self.stop_after {
                self.family = AssignmentMoveFamily::Done;
            }
            let family_limit =
                if self.spread_budget && self.options.max_moves > AssignmentMoveFamily::COUNT {
                    remaining.min(
                        self.options
                            .max_moves
                            .div_ceil(AssignmentMoveFamily::COUNT)
                            .max(1),
                    )
                } else {
                    remaining
                };
            let options = self.options.with_max_moves(family_limit);
            self.active_limit = family_limit;
            self.active_yielded = 0;
            self.active = match family {
                AssignmentMoveFamily::Required => {
                    if self.batch_required {
                        if let Some(mov) = self.required_batch_move(options) {
                            AssignmentFamilyCursor::Single(Some(mov))
                        } else {
                            AssignmentFamilyCursor::required_entity_values(
                                &self.group,
                                &self.solution,
                                &self.state,
                                options,
                            )
                        }
                    } else {
                        AssignmentFamilyCursor::entity_values(
                            ordered_entities(&self.group, &self.solution, |entity_index| {
                                self.state.is_required(entity_index)
                                    && self.state.current_value(entity_index).is_none()
                            }),
                            options,
                            AssignmentMoveKind::Required,
                        )
                    }
                }
                AssignmentMoveFamily::OptionalAssign => AssignmentFamilyCursor::entity_values(
                    ordered_entities(&self.group, &self.solution, |entity_index| {
                        !self.state.is_required(entity_index)
                            && self.state.current_value(entity_index).is_none()
                    }),
                    options,
                    AssignmentMoveKind::Optional,
                ),
                AssignmentMoveFamily::SequenceWindow => AssignmentFamilyCursor::PairWindow(
                    PairWindowCursor::sequence_window(&self.group, &self.solution, options),
                ),
                AssignmentMoveFamily::Rematch => AssignmentFamilyCursor::PairWindow(
                    PairWindowCursor::rematch(&self.group, &self.solution, &self.state, options),
                ),
                AssignmentMoveFamily::AugmentingRematch => {
                    AssignmentFamilyCursor::CycleWindow(CycleWindowCursor::augmenting(
                        &self.group,
                        &self.solution,
                        &self.state,
                        options,
                    ))
                }
                AssignmentMoveFamily::Swap => AssignmentFamilyCursor::PairWindow(
                    PairWindowCursor::swap(&self.group, &self.solution, &self.state, options),
                ),
                AssignmentMoveFamily::PairedReassignment => AssignmentFamilyCursor::PairWindow(
                    PairWindowCursor::paired(&self.group, &self.solution, &self.state, options),
                ),
                AssignmentMoveFamily::Reassignment => AssignmentFamilyCursor::entity_values(
                    ordered_entities(&self.group, &self.solution, |entity_index| {
                        self.state.current_value(entity_index).is_some()
                    }),
                    options,
                    AssignmentMoveKind::Reassignment,
                ),
                AssignmentMoveFamily::OptionalTransfer => {
                    AssignmentFamilyCursor::OptionalAdjustment(OptionalAdjustmentCursor::transfer(
                        &self.group,
                        &self.solution,
                        &self.state,
                        options,
                    ))
                }
                AssignmentMoveFamily::OptionalRelease => {
                    AssignmentFamilyCursor::OptionalAdjustment(OptionalAdjustmentCursor::release(
                        &self.group,
                        &self.solution,
                        &self.state,
                        options,
                    ))
                }
                AssignmentMoveFamily::EjectionReinsert => AssignmentFamilyCursor::CycleWindow(
                    CycleWindowCursor::ejection(&self.group, &self.solution, &self.state, options),
                ),
                AssignmentMoveFamily::Capacity => AssignmentFamilyCursor::Capacity(
                    CapacityCursor::new(&self.group, &self.solution, &self.state, options),
                ),
                AssignmentMoveFamily::Done => AssignmentFamilyCursor::Empty,
            };
            if !matches!(self.active, AssignmentFamilyCursor::Empty) {
                return true;
            }
        }
        false
    }

    fn required_batch_move(
        &mut self,
        options: ScalarAssignmentMoveOptions,
    ) -> Option<CompoundScalarMove<S>> {
        let entities = required_entities_by_scarcity(
            &self.group,
            &self.solution,
            &self.state,
            options.value_candidate_limit,
        );
        let value_degrees = required_value_degrees(
            &self.group,
            &self.solution,
            &entities,
            options.value_candidate_limit,
        );
        let mut scalar_edits = Vec::new();
        let mut edited_entities = HashSet::new();
        for entity_index in entities {
            if self.state.current_value(entity_index).is_some() {
                continue;
            }
            let mut values = self.group.candidate_values(
                &self.solution,
                entity_index,
                options.value_candidate_limit,
            );
            sort_values_by_required_scarcity(
                &self.group,
                &self.solution,
                entity_index,
                &value_degrees,
                &mut values,
            );
            for value in values {
                let Some(mov) = assignment_move_for_entity_value(
                    &self.group,
                    &self.solution,
                    &mut self.state,
                    entity_index,
                    value,
                    options,
                    AssignmentMoveIntent::required(),
                ) else {
                    continue;
                };
                if mov
                    .edits()
                    .iter()
                    .any(|edit| edited_entities.contains(&edit.entity_index))
                {
                    continue;
                }
                if mov.edits().len() != 1 {
                    continue;
                }
                for edit in mov.edits() {
                    self.state.set_value(
                        &self.group,
                        &self.solution,
                        edit.entity_index,
                        edit.to_value,
                    );
                    edited_entities.insert(edit.entity_index);
                    scalar_edits.push(self.group.edit(edit.entity_index, edit.to_value));
                }
                break;
            }
        }
        if scalar_edits.is_empty()
            || !self
                .state
                .scalar_edits_feasible(&self.group, &self.solution, &scalar_edits)
        {
            return None;
        }
        move_from_edits(
            &self.group,
            &self.solution,
            &scalar_edits,
            "scalar_assignment_required",
        )
    }

    fn next_family_move(&mut self) -> Option<CompoundScalarMove<S>> {
        match &mut self.active {
            AssignmentFamilyCursor::Single(mov) => {
                let candidate = mov.take();
                if candidate.is_none() {
                    self.active = AssignmentFamilyCursor::Empty;
                }
                candidate
            }
            AssignmentFamilyCursor::EntityValues(cursor) => {
                let candidate = cursor.next(&self.group, &self.solution, &mut self.state);
                if candidate.is_none() {
                    self.active = AssignmentFamilyCursor::Empty;
                }
                candidate
            }
            AssignmentFamilyCursor::Capacity(cursor) => {
                let candidate = cursor.next(&self.group, &self.solution, &mut self.state);
                if candidate.is_none() {
                    self.active = AssignmentFamilyCursor::Empty;
                }
                candidate
            }
            AssignmentFamilyCursor::OptionalAdjustment(cursor) => {
                let candidate = cursor.next(&self.group, &self.solution, &mut self.state);
                if candidate.is_none() {
                    self.active = AssignmentFamilyCursor::Empty;
                }
                candidate
            }
            AssignmentFamilyCursor::PairWindow(cursor) => {
                let candidate = cursor.next(&self.group, &self.solution, &mut self.state);
                if candidate.is_none() {
                    self.active = AssignmentFamilyCursor::Empty;
                }
                candidate
            }
            AssignmentFamilyCursor::CycleWindow(cursor) => {
                let candidate = cursor.next(&self.group, &self.solution, &mut self.state);
                if candidate.is_none() {
                    self.active = AssignmentFamilyCursor::Empty;
                }
                candidate
            }
            AssignmentFamilyCursor::Empty => None,
        }
    }
}

#[cfg(test)]
pub(crate) fn collect_assignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let mut cursor = ScalarAssignmentMoveCursor::new(*group, solution.clone(), options);
    let mut moves = Vec::new();
    while let Some(candidate) = cursor.next_move() {
        moves.push(candidate);
    }
    moves
}

#[cfg(test)]
pub(crate) fn rematch_assignment_moves<S>(
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    options: ScalarAssignmentMoveOptions,
) -> Vec<CompoundScalarMove<S>>
where
    S: PlanningSolution,
{
    let mut cursor = ScalarAssignmentMoveCursor::from_family_range(
        *group,
        solution.clone(),
        options,
        AssignmentMoveFamily::Rematch,
        AssignmentMoveFamily::Rematch,
        false,
        false,
    );
    let mut moves = Vec::new();
    while let Some(candidate) = cursor.next_move() {
        moves.push(candidate);
    }
    moves
}

fn normalized_move_key<S>(candidate: &CompoundScalarMove<S>) -> AssignmentMoveKey {
    let mut key = candidate
        .edits()
        .iter()
        .map(|edit| {
            (
                edit.descriptor_index,
                edit.entity_index,
                edit.variable_index,
                edit.variable_name,
                edit.to_value,
            )
        })
        .collect::<Vec<_>>();
    key.sort_unstable();
    key
}
