use std::collections::HashSet;

use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{ordered_entities, ScalarAssignmentMoveOptions};
use super::assignment_cycle::CycleWindowCursor;
use super::assignment_entity::{AssignmentMoveKind, CapacityCursor, OptionalAdjustmentCursor};
use super::assignment_family::{AssignmentFamilyCursor, AssignmentMoveFamily};
use super::assignment_pair::PairWindowCursor;
use super::assignment_required_batch::required_batch_move;
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
    state: ScalarAssignmentState,
    batch_required: bool,
    family_slots: Vec<AssignmentFamilySlot>,
    family_pos: usize,
    seen: HashSet<AssignmentMoveKey>,
    yielded: usize,
}

struct AssignmentFamilySlot {
    family: AssignmentMoveFamily,
    cursor: AssignmentFamilyCursor,
    exhausted: bool,
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

    pub(crate) fn construction_snapshot(&self) -> &S {
        &self.solution
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
        let families = if spread_budget {
            AssignmentMoveFamily::range(family, stop_after)
        } else {
            vec![family]
        };
        Self {
            state: ScalarAssignmentState::new(&group, &solution),
            group,
            solution,
            options,
            batch_required,
            family_slots: families
                .into_iter()
                .map(|family| AssignmentFamilySlot {
                    family,
                    cursor: AssignmentFamilyCursor::Empty,
                    exhausted: false,
                })
                .collect(),
            family_pos: 0,
            seen: HashSet::new(),
            yielded: 0,
        }
    }

    pub(crate) fn next_move_with_control<ShouldStop>(
        &mut self,
        should_stop: &mut ShouldStop,
    ) -> Option<CompoundScalarMove<S>>
    where
        ShouldStop: FnMut() -> bool,
    {
        if should_stop() {
            return None;
        }
        if self.options.max_moves == 0 || self.yielded >= self.options.max_moves {
            return None;
        }

        if self.batch_required {
            let candidate = required_batch_move(
                &self.group,
                &self.solution,
                &mut self.state,
                self.options,
                should_stop,
            );
            if let Some(candidate) = candidate {
                self.batch_required = false;
                self.seen.insert(normalized_move_key(&candidate));
                self.yielded += 1;
                return Some(candidate);
            }
            if should_stop() {
                return None;
            }
            self.batch_required = false;
        }

        let mut exhausted_turns = 0;
        while exhausted_turns < self.family_slots.len() {
            if should_stop() {
                return None;
            }
            let slot_index = self.family_pos % self.family_slots.len();
            self.family_pos = (slot_index + 1) % self.family_slots.len();
            if self.family_slots[slot_index].exhausted {
                exhausted_turns += 1;
                continue;
            }
            let Some(candidate) = self.next_slot_move(slot_index, should_stop) else {
                if should_stop() {
                    return None;
                }
                exhausted_turns += 1;
                continue;
            };
            exhausted_turns = 0;
            if !self.seen.insert(normalized_move_key(&candidate)) {
                continue;
            }
            self.yielded += 1;
            return Some(candidate);
        }
        None
    }

    fn next_slot_move<ShouldStop>(
        &mut self,
        slot_index: usize,
        should_stop: &mut ShouldStop,
    ) -> Option<CompoundScalarMove<S>>
    where
        ShouldStop: FnMut() -> bool,
    {
        if matches!(
            self.family_slots[slot_index].cursor,
            AssignmentFamilyCursor::Empty
        ) {
            let family = self.family_slots[slot_index].family;
            let cursor = self.open_family_cursor(family, self.options, should_stop)?;
            self.family_slots[slot_index].cursor = cursor;
            if matches!(
                self.family_slots[slot_index].cursor,
                AssignmentFamilyCursor::Empty
            ) {
                self.family_slots[slot_index].exhausted = true;
                return None;
            }
        }

        let candidate = {
            let cursor = &mut self.family_slots[slot_index].cursor;
            next_family_move(
                cursor,
                &self.group,
                &self.solution,
                &mut self.state,
                should_stop,
            )
        };
        if candidate.is_none() && !should_stop() {
            self.family_slots[slot_index].exhausted = true;
        }
        candidate
    }

    fn open_family_cursor<ShouldStop>(
        &mut self,
        family: AssignmentMoveFamily,
        options: ScalarAssignmentMoveOptions,
        should_stop: &mut ShouldStop,
    ) -> Option<AssignmentFamilyCursor>
    where
        ShouldStop: FnMut() -> bool,
    {
        if matches!(family, AssignmentMoveFamily::Required) {
            return AssignmentFamilyCursor::required_entity_values(
                &self.group,
                &self.solution,
                &self.state,
                options,
                should_stop,
            );
        }
        let cursor = match family {
            AssignmentMoveFamily::Required => unreachable!("required cursor opened above"),
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
            AssignmentMoveFamily::AugmentingRematch => AssignmentFamilyCursor::CycleWindow(
                CycleWindowCursor::augmenting(&self.group, &self.solution, &self.state, options),
            ),
            AssignmentMoveFamily::Swap => AssignmentFamilyCursor::PairWindow(
                PairWindowCursor::swap(&self.group, &self.solution, &self.state, options),
            ),
            AssignmentMoveFamily::PairedReassignment => AssignmentFamilyCursor::PairWindow(
                PairWindowCursor::paired(&self.group, &self.solution, &self.state, options),
            ),
            AssignmentMoveFamily::ValueWindowSwap => AssignmentFamilyCursor::ValueWindow(
                super::assignment_block::ValueWindowCursor::new(
                    &self.group,
                    &self.solution,
                    &self.state,
                    options,
                ),
            ),
            AssignmentMoveFamily::ValueLongWindowSwap => AssignmentFamilyCursor::ValueLongWindow(
                super::assignment_block::ValueLongWindowCursor::new(
                    &self.group,
                    &self.solution,
                    &self.state,
                    options,
                ),
            ),
            AssignmentMoveFamily::ValueRunGapSwap => AssignmentFamilyCursor::ValueRunGapSwap(
                super::assignment_value_run::ValueRunGapSwapCursor::new(
                    &self.group,
                    &self.solution,
                    &self.state,
                    options,
                ),
            ),
            AssignmentMoveFamily::ValueRunRelease => AssignmentFamilyCursor::ValueRunRelease(
                super::assignment_value_release::ValueRunReleaseCursor::new(
                    &self.group,
                    &self.solution,
                    &self.state,
                    options,
                ),
            ),
            AssignmentMoveFamily::ValueWindowCycle => AssignmentFamilyCursor::ValueWindowCycle(
                super::assignment_value_cycle::ValueWindowCycleCursor::new(
                    &self.group,
                    &self.solution,
                    &self.state,
                    options,
                ),
            ),
            AssignmentMoveFamily::ValueBlockReassignment => {
                AssignmentFamilyCursor::ValueBlockReassignment(
                    super::assignment_block::ValueBlockReassignmentCursor::new(
                        &self.group,
                        &self.solution,
                        &self.state,
                        options,
                    ),
                )
            }
            AssignmentMoveFamily::Reassignment => AssignmentFamilyCursor::entity_values(
                {
                    let mut entities =
                        ordered_entities(&self.group, &self.solution, |entity_index| {
                            self.state.current_value(entity_index).is_some()
                        });
                    self.state.sort_entities_by_current_value_pressure(
                        &self.group,
                        &self.solution,
                        &mut entities,
                    );
                    entities
                },
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
        if should_stop() {
            None
        } else {
            Some(cursor)
        }
    }
}

fn next_family_move<S, ShouldStop>(
    cursor: &mut AssignmentFamilyCursor,
    group: &ScalarAssignmentBinding<S>,
    solution: &S,
    state: &mut ScalarAssignmentState,
    should_stop: &mut ShouldStop,
) -> Option<CompoundScalarMove<S>>
where
    S: PlanningSolution,
    ShouldStop: FnMut() -> bool,
{
    if should_stop() {
        return None;
    }
    match cursor {
        AssignmentFamilyCursor::EntityValues(entity_cursor) => {
            let candidate = entity_cursor.next(group, solution, state, should_stop);
            if candidate.is_none() && !should_stop() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::Capacity(capacity_cursor) => {
            let candidate = capacity_cursor.next(group, solution, state, should_stop);
            if candidate.is_none() && !should_stop() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::OptionalAdjustment(optional_cursor) => {
            let candidate = optional_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::PairWindow(pair_cursor) => {
            let candidate = pair_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::CycleWindow(cycle_cursor) => {
            let candidate = cycle_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueWindow(value_window_cursor) => {
            let candidate = value_window_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueLongWindow(long_window_cursor) => {
            let candidate = long_window_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueRunGapSwap(run_gap_cursor) => {
            let candidate = run_gap_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueRunRelease(run_release_cursor) => {
            let candidate = run_release_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueWindowCycle(value_window_cycle_cursor) => {
            let candidate = value_window_cycle_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::ValueBlockReassignment(block_cursor) => {
            let candidate = block_cursor.next(group, solution, state);
            if candidate.is_none() {
                *cursor = AssignmentFamilyCursor::Empty;
            }
            candidate
        }
        AssignmentFamilyCursor::Empty => None,
    }
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
