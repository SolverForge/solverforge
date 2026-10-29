//! Change emitter over a dynamic host list slot, for the reference selectors in tests.
use super::ChangeEmitter;

use crate::heuristic::r#move::DynamicListChangeMove;
use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};
use std::fmt::{self, Debug};

/// Dynamic relocation emission preserving DynamicListChangeMove ownership.
#[derive(Clone)]
pub(crate) struct DynamicChangeEmitter<S> {
    slot: DynamicListVariableSlot<S>,
}

impl<S> DynamicChangeEmitter<S> {
    pub(crate) fn new(slot: DynamicListVariableSlot<S>) -> Self {
        Self { slot }
    }
}

impl<S> Debug for DynamicChangeEmitter<S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DynamicChangeEmitter")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<S> ChangeEmitter<S> for DynamicChangeEmitter<S>
where
    S: PlanningSolution,
{
    type Move = DynamicListChangeMove<S>;

    fn emit_change(
        &self,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move {
        DynamicListChangeMove::new(
            self.slot.clone(),
            source_entity,
            source_position,
            destination_entity,
            destination_position,
        )
    }
}
