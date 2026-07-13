//! Physical emission adapters for reverse and contiguous-window moves.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::{
    ListPermuteMove, ListReverseMove, Move, SublistChangeMove, SublistSwapMove,
    MAX_LIST_PERMUTE_WINDOW_SIZE,
};

pub(crate) trait SublistChangeEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_sublist_change(
        &self,
        source_entity: usize,
        source_start: usize,
        source_end: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move;
}

pub(crate) trait SublistSwapEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_sublist_swap(
        &self,
        first_entity: usize,
        first_start: usize,
        first_end: usize,
        second_entity: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self::Move;
}

pub(crate) trait ReverseEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_reverse(&self, entity: usize, start: usize, end: usize) -> Self::Move;
}

pub(crate) trait PermuteEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_permute(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> Self::Move;
}

/// Static emission preserving ListReverseMove function-pointer behavior.
#[derive(Clone, Copy)]
pub(crate) struct NativeReverseEmitter<S, V> {
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_reverse: fn(&mut S, usize, usize, usize),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> NativeReverseEmitter<S, V> {
    pub(crate) fn new(
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_reverse: fn(&mut S, usize, usize, usize),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            list_get,
            list_reverse,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for NativeReverseEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeReverseEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> ReverseEmitter<S> for NativeReverseEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Move = ListReverseMove<S, V>;

    fn emit_reverse(&self, entity: usize, start: usize, end: usize) -> Self::Move {
        ListReverseMove::new(
            entity,
            start,
            end,
            self.list_len,
            self.list_get,
            self.list_reverse,
            self.variable_name,
            self.descriptor_index,
        )
    }
}

/// Static emission preserving ListPermuteMove contiguous-window mutation.
#[derive(Clone, Copy)]
pub(crate) struct NativeWindowEmitter<S, V> {
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> NativeWindowEmitter<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            list_len,
            list_get,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for NativeWindowEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeWindowEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> SublistChangeEmitter<S> for NativeWindowEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Move = SublistChangeMove<S, V>;

    fn emit_sublist_change(
        &self,
        source_entity: usize,
        source_start: usize,
        source_end: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move {
        SublistChangeMove::new(
            source_entity,
            source_start,
            source_end,
            destination_entity,
            destination_position,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }
}

impl<S, V> SublistSwapEmitter<S> for NativeWindowEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Move = SublistSwapMove<S, V>;

    fn emit_sublist_swap(
        &self,
        first_entity: usize,
        first_start: usize,
        first_end: usize,
        second_entity: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self::Move {
        SublistSwapMove::new(
            first_entity,
            first_start,
            first_end,
            second_entity,
            second_start,
            second_end,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }
}

impl<S, V> PermuteEmitter<S> for NativeWindowEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Move = ListPermuteMove<S, V>;

    fn emit_permute(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> Self::Move {
        ListPermuteMove::new(
            entity,
            start,
            end,
            permutation,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        )
    }
}
