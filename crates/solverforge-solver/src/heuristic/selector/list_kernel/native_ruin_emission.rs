//! Ruin emitter over static list accessors, for the reference selectors in tests.
use super::RuinEmitter;

use crate::heuristic::r#move::ListRuinMove;
use solverforge_core::domain::PlanningSolution;
use std::fmt::{self, Debug};
use std::marker::PhantomData;

/// Native public move emitter for [`ListRuinMove`].
#[derive(Clone, Copy)]
pub(crate) struct NativeRuinEmitter<S, V> {
    entity_count: fn(&S) -> usize,
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    list_remove: fn(&mut S, usize, usize) -> V,
    list_insert: fn(&mut S, usize, usize, V),
    element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
    skip_empty_destinations: bool,
    variable_name: &'static str,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> NativeRuinEmitter<S, V> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        entity_count: fn(&S) -> usize,
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_remove: fn(&mut S, usize, usize) -> V,
        list_insert: fn(&mut S, usize, usize, V),
        element_owner_fn: Option<fn(&S, &V) -> Option<usize>>,
        skip_empty_destinations: bool,
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            entity_count,
            list_len,
            list_get,
            list_remove,
            list_insert,
            element_owner_fn,
            skip_empty_destinations,
            variable_name,
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for NativeRuinEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeRuinEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> RuinEmitter<S> for NativeRuinEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Move = ListRuinMove<S, V>;

    fn emit_ruin(&self, entity: usize, indices: &[usize]) -> Self::Move {
        ListRuinMove::new(
            entity,
            indices,
            self.entity_count,
            self.list_len,
            self.list_get,
            self.list_remove,
            self.list_insert,
            self.variable_name,
            self.descriptor_index,
        )
        .with_element_owner_fn(self.element_owner_fn)
        .with_skip_empty_destinations(self.skip_empty_destinations)
    }
}
