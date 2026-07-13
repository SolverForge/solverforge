//! Physical move emission for the shared full-list cursors.
//!
//! Cursors choose coordinates only.  These adapters preserve the existing
//! static and dynamic public move carriers without adding a second stream.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::{DynamicListVariableSlot, PlanningSolution};

use crate::heuristic::r#move::{DynamicListChangeMove, ListChangeMove, ListSwapMove, Move};

/// Emits one relocation move for already-approved coordinates.
pub(crate) trait ChangeEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_change(
        &self,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move;
}

/// Emits one exchange move for already-approved coordinates.
pub(crate) trait SwapEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_swap(
        &self,
        first_entity: usize,
        first_position: usize,
        second_entity: usize,
        second_position: usize,
    ) -> Self::Move;
}

/// Static relocation emission preserving ListChangeMove function pointers.
#[derive(Clone, Copy)]
pub(crate) struct NativeChangeEmitter<S, V> {
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> NativeChangeEmitter<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            list_get,
            list_remove,
            list_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for NativeChangeEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeChangeEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ChangeEmitter<S> for NativeChangeEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Move = ListChangeMove<S, V>;

    fn emit_change(
        &self,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move {
        ListChangeMove::new(
            source_entity,
            source_position,
            destination_entity,
            destination_position,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }
}

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

/// Static exchange emission preserving ListSwapMove function pointers.
#[derive(Clone, Copy)]
pub(crate) struct NativeSwapEmitter<S, V> {
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_set: fn(&mut S, usize, usize, V),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> NativeSwapEmitter<S, V> {
    pub(crate) fn new(
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            list_get,
            list_set,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for NativeSwapEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeSwapEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> SwapEmitter<S> for NativeSwapEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Move = ListSwapMove<S, V>;

    fn emit_swap(
        &self,
        first_entity: usize,
        first_position: usize,
        second_entity: usize,
        second_position: usize,
    ) -> Self::Move {
        ListSwapMove::new(
            first_entity,
            first_position,
            second_entity,
            second_position,
            self.list_len,
            self.list_get,
            self.list_set,
            self.variable_name,
            self.descriptor_index,
        )
    }
}
