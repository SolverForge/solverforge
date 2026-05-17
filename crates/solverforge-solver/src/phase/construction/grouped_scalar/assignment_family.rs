use std::collections::HashMap;

use solverforge_core::domain::PlanningSolution;

use super::assignment_block::{
    ValueBlockReassignmentCursor, ValueLongWindowCursor, ValueWindowCursor,
};
use super::assignment_candidate::{rotate_entity_order, ScalarAssignmentMoveOptions};
use super::assignment_cycle::CycleWindowCursor;
use super::assignment_entity::{
    required_entities_by_scarcity, required_value_degrees, AssignmentMoveKind, CapacityCursor,
    EntityValueCursor, OptionalAdjustmentCursor,
};
use super::assignment_pair::PairWindowCursor;
use super::assignment_state::ScalarAssignmentState;
use super::assignment_value_cycle::ValueWindowCycleCursor;
use super::assignment_value_release::ValueRunReleaseCursor;
use super::assignment_value_run::ValueRunGapSwapCursor;
use crate::builder::ScalarAssignmentBinding;

pub(super) enum AssignmentFamilyCursor {
    EntityValues(EntityValueCursor),
    Capacity(CapacityCursor),
    OptionalAdjustment(OptionalAdjustmentCursor),
    PairWindow(PairWindowCursor),
    CycleWindow(CycleWindowCursor),
    ValueWindow(ValueWindowCursor),
    ValueLongWindow(ValueLongWindowCursor),
    ValueRunGapSwap(ValueRunGapSwapCursor),
    ValueRunRelease(ValueRunReleaseCursor),
    ValueWindowCycle(ValueWindowCycleCursor),
    ValueBlockReassignment(ValueBlockReassignmentCursor),
    Empty,
}

impl AssignmentFamilyCursor {
    pub(super) fn required_entity_values<S>(
        group: &ScalarAssignmentBinding<S>,
        solution: &S,
        state: &ScalarAssignmentState,
        options: ScalarAssignmentMoveOptions,
    ) -> Self
    where
        S: PlanningSolution,
    {
        let mut entities =
            required_entities_by_scarcity(group, solution, state, options.value_candidate_limit);
        rotate_entity_order(&mut entities, options.entity_offset);
        let value_degrees =
            required_value_degrees(group, solution, &entities, options.value_candidate_limit);
        Self::EntityValues(EntityValueCursor {
            entities,
            entity_pos: 0,
            values: Vec::new(),
            value_pos: 0,
            value_degrees,
            options,
            kind: AssignmentMoveKind::Required,
        })
    }

    pub(super) fn entity_values(
        mut entities: Vec<usize>,
        options: ScalarAssignmentMoveOptions,
        kind: AssignmentMoveKind,
    ) -> Self {
        rotate_entity_order(&mut entities, options.entity_offset);
        Self::EntityValues(EntityValueCursor {
            entities,
            entity_pos: 0,
            values: Vec::new(),
            value_pos: 0,
            value_degrees: HashMap::new(),
            options,
            kind,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AssignmentMoveFamily {
    Required,
    Capacity,
    ValueRunGapSwap,
    ValueRunRelease,
    SequenceWindow,
    Rematch,
    AugmentingRematch,
    Swap,
    PairedReassignment,
    ValueWindowSwap,
    ValueLongWindowSwap,
    ValueWindowCycle,
    ValueBlockReassignment,
    Reassignment,
    OptionalTransfer,
    OptionalAssign,
    OptionalRelease,
    EjectionReinsert,
    Done,
}

impl AssignmentMoveFamily {
    pub(super) fn take_next(&mut self) -> Option<Self> {
        let current = *self;
        *self = match current {
            Self::Required => Self::Capacity,
            Self::Capacity => Self::ValueRunGapSwap,
            Self::ValueRunGapSwap => Self::ValueRunRelease,
            Self::ValueRunRelease => Self::SequenceWindow,
            Self::SequenceWindow => Self::Rematch,
            Self::Rematch => Self::AugmentingRematch,
            Self::AugmentingRematch => Self::Swap,
            Self::Swap => Self::PairedReassignment,
            Self::PairedReassignment => Self::ValueWindowSwap,
            Self::ValueWindowSwap => Self::ValueLongWindowSwap,
            Self::ValueLongWindowSwap => Self::ValueBlockReassignment,
            Self::ValueBlockReassignment => Self::ValueWindowCycle,
            Self::ValueWindowCycle => Self::Reassignment,
            Self::Reassignment => Self::OptionalTransfer,
            Self::OptionalTransfer => Self::OptionalAssign,
            Self::OptionalAssign => Self::OptionalRelease,
            Self::OptionalRelease => Self::EjectionReinsert,
            Self::EjectionReinsert | Self::Done => Self::Done,
        };
        (current != Self::Done).then_some(current)
    }

    pub(super) fn range(start: Self, stop_after: Self) -> Vec<Self> {
        let mut cursor = start;
        let mut families = Vec::new();
        while let Some(family) = cursor.take_next() {
            families.push(family);
            if family == stop_after {
                break;
            }
        }
        families
    }
}
