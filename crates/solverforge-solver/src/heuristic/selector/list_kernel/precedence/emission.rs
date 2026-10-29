//! Physical move emission for canonical precedence coordinates.

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::{Move, MAX_LIST_PERMUTE_WINDOW_SIZE};

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
