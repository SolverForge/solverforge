use solverforge_core::domain::PlanningSolution;

use super::assignment_candidate::{
    normalized_move_key, ordered_entities, AssignmentMoveKey, ScalarAssignmentMoveOptions,
};
use super::assignment_cycle::CycleWindowCursor;
use super::assignment_entity::{AssignmentMoveKind, CapacityCursor, OptionalAdjustmentCursor};
use super::assignment_family::{AssignmentFamilyCursor, AssignmentMoveFamily};
use super::assignment_pair::PairWindowCursor;
use super::assignment_required_batch::required_batch_move;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_index::{assigned_value_sequence_index, AssignedValueSequenceIndex};
use crate::builder::ScalarAssignmentBinding;
use crate::heuristic::r#move::CompoundScalarMove;
use crate::phase::construction::ConstructionSlotId;
use solverforge_core::fast_hash::FastSet;

pub(crate) struct ScalarAssignmentMoveCursor<S>
where
    S: PlanningSolution,
{
    marker: std::marker::PhantomData<fn() -> S>,
    options: ScalarAssignmentMoveOptions,
    state: ScalarAssignmentState,
    // Built lazily on the first value-driven family open of a step; every
    // candidate build rolls the state back, so the index content is
    // identical for every open within one step.
    index_cache: Option<AssignedValueSequenceIndex>,
    batch_required: bool,
    family_slots: Vec<AssignmentFamilySlot>,
    family_pos: usize,
    seen: FastSet<AssignmentMoveKey>,
    yielded: usize,
}

mod dispatch;
use dispatch::next_family_move;

#[cfg(test)]
#[path = "tests/index_laziness.rs"]
mod index_laziness;

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
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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

    /// Step cursor seeded from the solve-owned retained base. The state moves
    /// into the cursor for the step and is handed back through `into_state`.
    pub(crate) fn with_pooled_state(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        options: ScalarAssignmentMoveOptions,
        state: ScalarAssignmentState,
    ) -> Self {
        let mut cursor = Self::from_family_range(
            group,
            solution,
            options,
            AssignmentMoveFamily::Required,
            AssignmentMoveFamily::EjectionReinsert,
            true,
            false,
        );
        cursor.state = state;
        cursor
    }

    /// Moves the maintained assignment base out of the step cursor so the
    /// solve-owned slot can retain it for the next step.
    pub(crate) fn into_state(self) -> ScalarAssignmentState {
        self.state
    }

    pub(crate) fn required_construction(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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
    pub(super) fn exclude_completed_optional_entities(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        completed: &FastSet<ConstructionSlotId>,
    ) {
        if self.family_slots.len() != 1
            || self.family_slots[0].family != AssignmentMoveFamily::OptionalAssign
        {
            return;
        }
        // Keep occupancy intact: completed rows can still block an assignment.
        // Exclude roots before generation so they cannot consume the move budget.
        let binding_index = group.target().construction_binding_index();
        self.family_slots[0].cursor = AssignmentFamilyCursor::optional_entity_values(
            group,
            solution,
            &self.state,
            self.options,
            |entity| completed.contains(&ConstructionSlotId::new(binding_index, entity)),
        );
    }
    pub(crate) fn required(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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
            state: ScalarAssignmentState::new(group, solution),
            marker: std::marker::PhantomData,
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
            index_cache: None,
            seen: FastSet::default(),
            yielded: 0,
        }
    }

    pub(crate) fn next_move_with_control<ShouldStop>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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
            let candidate =
                required_batch_move(group, solution, &mut self.state, self.options, should_stop);
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
            let Some(candidate) = self.next_slot_move(group, solution, slot_index, should_stop)
            else {
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
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
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
            let cursor =
                self.open_family_cursor(group, solution, family, self.options, should_stop)?;
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
            let index = self.index_cache.as_ref();
            next_family_move(cursor, index, group, solution, &mut self.state, should_stop)
        };
        if candidate.is_none() && !should_stop() {
            self.family_slots[slot_index].exhausted = true;
        }
        candidate
    }

    /// Builds the assigned value/sequence index once per step. Family opens
    /// and pulls borrow it; no family cursor owns a copy any more. Builds
    /// are pure reads of the committed solution/state (every candidate
    /// build rolls back), so the content is identical across opens within
    /// a step.
    fn build_index_if_needed(&mut self, group: &ScalarAssignmentBinding<S>, solution: &S) {
        if self.index_cache.is_none() {
            self.index_cache = Some(assigned_value_sequence_index(
                group,
                solution,
                &self.state,
                self.options,
            ));
        }
    }

    fn open_family_cursor<ShouldStop>(
        &mut self,
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        family: AssignmentMoveFamily,
        options: ScalarAssignmentMoveOptions,
        should_stop: &mut ShouldStop,
    ) -> Option<AssignmentFamilyCursor>
    where
        ShouldStop: FnMut() -> bool,
    {
        if matches!(family, AssignmentMoveFamily::Required) {
            return AssignmentFamilyCursor::required_entity_values(
                group,
                solution,
                &self.state,
                options,
                should_stop,
            );
        }
        let cursor = match family {
            AssignmentMoveFamily::Required => unreachable!("required cursor opened above"),
            AssignmentMoveFamily::OptionalAssign => AssignmentFamilyCursor::optional_entity_values(
                group,
                solution,
                &self.state,
                options,
                |_| false,
            ),
            AssignmentMoveFamily::SequenceWindow => AssignmentFamilyCursor::PairWindow(
                PairWindowCursor::sequence_window(group, solution, options),
            ),
            AssignmentMoveFamily::Rematch => AssignmentFamilyCursor::PairWindow(
                PairWindowCursor::rematch(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::AugmentingRematch => AssignmentFamilyCursor::CycleWindow(
                CycleWindowCursor::augmenting(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::Swap => AssignmentFamilyCursor::PairWindow(
                PairWindowCursor::swap(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::PairedReassignment => AssignmentFamilyCursor::PairWindow(
                PairWindowCursor::paired(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::ValueWindowSwap => {
                self.build_index_if_needed(group, solution);
                AssignmentFamilyCursor::ValueWindow(
                    super::assignment_block::ValueWindowCursor::new(
                        options,
                        self.index_cache.as_ref().expect("just built"),
                    ),
                )
            }
            AssignmentMoveFamily::ValueLongWindowSwap => {
                self.build_index_if_needed(group, solution);
                AssignmentFamilyCursor::ValueLongWindow(
                    super::assignment_block::ValueLongWindowCursor::new(
                        options,
                        self.index_cache.as_ref().expect("just built"),
                    ),
                )
            }
            AssignmentMoveFamily::ValueRunGapSwap => {
                self.build_index_if_needed(group, solution);
                AssignmentFamilyCursor::ValueRunGapSwap(
                    super::assignment_value_run::ValueRunGapSwapCursor::new(
                        options,
                        self.index_cache.as_ref().expect("just built"),
                    ),
                )
            }
            AssignmentMoveFamily::ValueRunRelease => {
                self.build_index_if_needed(group, solution);
                AssignmentFamilyCursor::ValueRunRelease(
                    super::assignment_value_release::ValueRunReleaseCursor::new(
                        options,
                        self.index_cache.as_ref().expect("just built"),
                    ),
                )
            }
            AssignmentMoveFamily::ValueWindowCycle => {
                self.build_index_if_needed(group, solution);
                AssignmentFamilyCursor::ValueWindowCycle(
                    super::assignment_value_cycle::ValueWindowCycleCursor::new(
                        options,
                        self.index_cache.as_ref().expect("just built"),
                    ),
                )
            }
            AssignmentMoveFamily::ValueBlockReassignment => {
                self.build_index_if_needed(group, solution);
                AssignmentFamilyCursor::ValueBlockReassignment(
                    super::assignment_block::ValueBlockReassignmentCursor::new(
                        options,
                        self.index_cache.as_ref().expect("just built"),
                    ),
                )
            }
            AssignmentMoveFamily::Reassignment => AssignmentFamilyCursor::entity_values(
                {
                    let mut entities = ordered_entities(group, solution, |entity_index| {
                        self.state.current_value(entity_index).is_some()
                    });
                    self.state.sort_entities_by_current_value_pressure(
                        group,
                        solution,
                        &mut entities,
                    );
                    entities
                },
                options,
                AssignmentMoveKind::Reassignment,
            ),
            AssignmentMoveFamily::OptionalTransfer => AssignmentFamilyCursor::OptionalAdjustment(
                OptionalAdjustmentCursor::transfer(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::OptionalRelease => AssignmentFamilyCursor::OptionalAdjustment(
                OptionalAdjustmentCursor::release(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::EjectionReinsert => AssignmentFamilyCursor::CycleWindow(
                CycleWindowCursor::ejection(group, solution, &self.state, options),
            ),
            AssignmentMoveFamily::Capacity => AssignmentFamilyCursor::Capacity(
                CapacityCursor::new(group, solution, &self.state, options),
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
