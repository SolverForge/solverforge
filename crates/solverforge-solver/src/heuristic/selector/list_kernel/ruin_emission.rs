//! Physical emission for canonical list-ruin coordinates.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use crate::heuristic::r#move::Move;

/// Converts a selected ruin coordinate set into its concrete move carrier.
///
/// Enumeration, random draw order, and candidate ownership live in
/// [`super::RuinCursor`]; emitters may not filter or reorder coordinates.
pub(crate) trait RuinEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_ruin(&self, entity: usize, indices: &[usize]) -> Self::Move;
}
