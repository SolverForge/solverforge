//! Physical emission for canonical list-ruin coordinates.

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;

use smallvec::SmallVec;

use crate::heuristic::r#move::list_kernel::RuinSources;
use crate::heuristic::r#move::Move;

/// Converts a selected ruin coordinate set into its concrete move carrier.
///
/// Enumeration, random draw order, and candidate ownership live in
/// [`super::RuinCursor`]; emitters may not filter or reorder coordinates.
pub(crate) trait RuinEmitter<S: PlanningSolution>: Send + Sync + Debug {
    type Move: Move<S>;

    fn emit_ruin(&self, entity: usize, indices: SmallVec<[usize; 8]>) -> Self::Move;
}

/// Converts ruin coordinates spanning several lists into one move carrier.
///
/// `sources` holds distinct entities in ascending order, each with ascending
/// distinct positions; the emitter takes it by ownership.
pub(crate) trait MultiRuinEmitter<S: PlanningSolution>: RuinEmitter<S> {
    fn emit_ruin_sources(
        &self,
        sources: RuinSources,
        max_count: usize,
        max_routes: usize,
        eligible: impl Fn(usize, usize) -> bool,
    ) -> Option<Self::Move>;
}
