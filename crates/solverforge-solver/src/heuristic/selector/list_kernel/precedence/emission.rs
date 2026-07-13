//! Physical move emission for canonical precedence coordinates.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::{
    ListChangeMove, ListMoveUnion, ListMultiSwapMove, ListPermuteMove, ListReverseMove,
    ListRuinMove, ListSwapMove, Move, SublistChangeMove, SublistSwapMove,
    MAX_LIST_PERMUTE_WINDOW_SIZE,
};

/// Converts already-enumerated precedence coordinates into one move carrier.
///
/// The cursor owns ordering, tiering, and cycle pruning; an emitter may not
/// add filtering or an alternate candidate store.
pub(crate) trait PrecedenceEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_change(&self, entity: usize, source: usize, destination: usize) -> Self::Move;
    fn emit_swap(&self, entity: usize, first: usize, second: usize) -> Self::Move;
    fn emit_reverse(&self, entity: usize, start: usize, end: usize) -> Self::Move;
    fn emit_sublist_swap(
        &self,
        entity: usize,
        first_start: usize,
        first_end: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self::Move;
    fn emit_ruin(&self, sources: &[(usize, SmallVec<[usize; 8]>)]) -> Self::Move;
    fn emit_sublist_change(
        &self,
        entity: usize,
        source_start: usize,
        source_end: usize,
        destination: usize,
    ) -> Self::Move;
    fn emit_permute(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> Self::Move;
    fn emit_multi_swap(&self, swaps: &[(usize, usize, usize)]) -> Self::Move;
}

/// Native public-union emitter for the precedence cursor.
#[derive(Clone, Copy)]
pub(crate) struct NativePrecedenceEmitter<S, V> {
    element_count: fn(&S) -> usize,
    index_to_element: fn(&S, usize) -> V,
    fixed_successors: fn(&S, V, &mut Vec<V>),
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> Option<V>,
    list_insert: fn(&mut S, usize, usize, V),
    list_set: fn(&mut S, usize, usize, V),
    list_reverse: fn(&mut S, usize, usize, usize),
    ruin_remove: fn(&mut S, usize, usize) -> V,
    ruin_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> S>,
}

impl<S, V> NativePrecedenceEmitter<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        element_count: fn(&S) -> usize,
        index_to_element: fn(&S, usize) -> V,
        fixed_successors: fn(&S, V, &mut Vec<V>),
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> Option<V>,
        list_insert: fn(&mut S, usize, usize, V),
        list_set: fn(&mut S, usize, usize, V),
        list_reverse: fn(&mut S, usize, usize, usize),
        ruin_remove: fn(&mut S, usize, usize) -> V,
        ruin_insert: fn(&mut S, usize, usize, V),
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
        sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
        sublist_insert: fn(&mut S, usize, usize, Vec<V>),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            element_count,
            index_to_element,
            fixed_successors,
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            list_set,
            list_reverse,
            ruin_remove,
            ruin_insert,
            element_owner_fn,
            sublist_remove,
            sublist_insert,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }

    fn precedence_ruin(&self, sources: &[(usize, SmallVec<[usize; 8]>)]) -> ListRuinMove<S, V> {
        let move_ = if let [(entity, indices)] = sources {
            ListRuinMove::new(
                *entity,
                indices,
                self.entity_count,
                self.list_len,
                self.list_get,
                self.ruin_remove,
                self.ruin_insert,
                self.variable_name,
                self.descriptor_index,
            )
        } else {
            ListRuinMove::new_multi_source(
                sources,
                self.entity_count,
                self.list_len,
                self.list_get,
                self.ruin_remove,
                self.ruin_insert,
                self.variable_name,
                self.descriptor_index,
            )
        };
        move_
            .with_element_owner_fn(self.element_owner_fn)
            .with_precedence_hooks(
                Some(self.element_count),
                Some(self.index_to_element),
                Some(self.fixed_successors),
            )
    }
}

impl<S, V> Debug for NativePrecedenceEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativePrecedenceEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> PrecedenceEmitter<S> for NativePrecedenceEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Move = ListMoveUnion<S, V>;

    fn emit_change(&self, entity: usize, source: usize, destination: usize) -> Self::Move {
        ListMoveUnion::ListChange(ListChangeMove::new(
            entity,
            source,
            entity,
            destination,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn emit_swap(&self, entity: usize, first: usize, second: usize) -> Self::Move {
        ListMoveUnion::ListSwap(ListSwapMove::new(
            entity,
            first,
            entity,
            second,
            self.list_len,
            self.list_get,
            self.list_set,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn emit_reverse(&self, entity: usize, start: usize, end: usize) -> Self::Move {
        ListMoveUnion::ListReverse(ListReverseMove::new(
            entity,
            start,
            end,
            self.list_len,
            self.list_get,
            self.list_reverse,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn emit_sublist_swap(
        &self,
        entity: usize,
        first_start: usize,
        first_end: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self::Move {
        ListMoveUnion::SublistSwap(SublistSwapMove::new(
            entity,
            first_start,
            first_end,
            entity,
            second_start,
            second_end,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn emit_ruin(&self, sources: &[(usize, SmallVec<[usize; 8]>)]) -> Self::Move {
        ListMoveUnion::ListRuin(self.precedence_ruin(sources))
    }

    fn emit_sublist_change(
        &self,
        entity: usize,
        source_start: usize,
        source_end: usize,
        destination: usize,
    ) -> Self::Move {
        ListMoveUnion::SublistChange(SublistChangeMove::new(
            entity,
            source_start,
            source_end,
            entity,
            destination,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.descriptor_index,
        ))
    }

    fn emit_permute(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> Self::Move {
        ListMoveUnion::ListPermute(ListPermuteMove::new(
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
        ))
    }

    fn emit_multi_swap(&self, swaps: &[(usize, usize, usize)]) -> Self::Move {
        ListMoveUnion::ListMultiSwap(
            ListMultiSwapMove::new(
                swaps,
                self.list_len,
                self.list_get,
                self.list_set,
                self.variable_name,
                self.descriptor_index,
            )
            .with_require_score_improvement(true),
        )
    }
}
