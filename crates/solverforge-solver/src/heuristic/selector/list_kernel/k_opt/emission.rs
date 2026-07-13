//! Physical K-opt move emission for the shared cursor kernels.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::r#move::metadata::hash_str;
use crate::heuristic::r#move::{CutPoint, KOptMove, Move};

pub(crate) trait KOptEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_k_opt(&self, cuts: &[CutPoint], reconnection: &KOptReconnection) -> Self::Move;
}

/// Static emission preserving the public `KOptMove<S, V>` function-pointer
/// carrier while the shared cursors retain coordinate and pattern ownership.
#[derive(Clone, Copy)]
pub(crate) struct NativeKOptEmitter<S, V> {
    list_len: fn(&S, usize) -> usize,
    list_get: fn(&S, usize, usize) -> Option<V>,
    sublist_remove: fn(&mut S, usize, usize, usize) -> Vec<V>,
    sublist_insert: fn(&mut S, usize, usize, Vec<V>),
    variable_name: &'static str,
    variable_id: u64,
    descriptor_index: usize,
    _phantom: PhantomData<fn() -> (S, V)>,
}

impl<S, V> NativeKOptEmitter<S, V> {
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
            variable_id: hash_str(variable_name),
            descriptor_index,
            _phantom: PhantomData,
        }
    }
}

impl<S, V> Debug for NativeKOptEmitter<S, V> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeKOptEmitter")
            .field("variable_name", &self.variable_name)
            .field("descriptor_index", &self.descriptor_index)
            .finish()
    }
}

impl<S, V> KOptEmitter<S> for NativeKOptEmitter<S, V>
where
    S: PlanningSolution,
    V: Clone + Send + Sync + Debug + 'static,
{
    type Move = KOptMove<S, V>;

    fn emit_k_opt(&self, cuts: &[CutPoint], reconnection: &KOptReconnection) -> Self::Move {
        KOptMove::new_with_variable_id(
            cuts,
            reconnection,
            self.list_len,
            self.list_get,
            self.sublist_remove,
            self.sublist_insert,
            self.variable_name,
            self.variable_id,
            self.descriptor_index,
        )
    }
}
