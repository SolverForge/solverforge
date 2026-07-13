//! Public static facade for canonical multi-list-swap mechanics.

use std::fmt::Debug;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use super::list_kernel::{
    multi_swap_do_move, multi_swap_entity_indices, multi_swap_is_doable, multi_swap_tabu_signature,
    multi_swap_undo_move, MultiSwapCoordinates, StaticListSwapAccess,
};
use super::{Move, MoveTabuSignature};

/// Multiple independent intra-list swaps, one per entity.
///
/// The public move keeps its historic constructor and score-improvement flag;
/// shared list-kernel mechanics own doability, notifications, mutation, undo,
/// and tabu identity for static and runtime carriers alike.
pub struct ListMultiSwapMove<S, V> {
    swaps: MultiSwapCoordinates,
    access: StaticListSwapAccess<S, V>,
    indices: SmallVec<[usize; 4]>,
    require_score_improvement: bool,
}

impl<S, V> Clone for ListMultiSwapMove<S, V> {
    fn clone(&self) -> Self {
        Self {
            swaps: self.swaps.clone(),
            access: self.access,
            indices: self.indices.clone(),
            require_score_improvement: self.require_score_improvement,
        }
    }
}

impl<S, V: Debug> Debug for ListMultiSwapMove<S, V> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ListMultiSwapMove")
            .field("swaps", &self.swaps)
            .field("variable_name", &self.access.variable_name)
            .field("require_score_improvement", &self.require_score_improvement)
            .finish()
    }
}

impl<S, V> ListMultiSwapMove<S, V> {
    pub fn new(
        swaps: &[(usize, usize, usize)],
        list_len: fn(&S, usize) -> usize,
        list_get: fn(&S, usize, usize) -> Option<V>,
        list_set: fn(&mut S, usize, usize, V),
        variable_name: &'static str,
        descriptor_index: usize,
    ) -> Self {
        Self {
            swaps: swaps.iter().copied().collect(),
            access: StaticListSwapAccess {
                list_len,
                list_get,
                list_set,
                variable_name,
                descriptor_index,
            },
            indices: multi_swap_entity_indices(swaps),
            require_score_improvement: false,
        }
    }

    pub fn with_require_score_improvement(mut self, require_score_improvement: bool) -> Self {
        self.require_score_improvement = require_score_improvement;
        self
    }

    pub fn swaps(&self) -> &[(usize, usize, usize)] {
        &self.swaps
    }
}

impl<S, V> Move<S> for ListMultiSwapMove<S, V>
where
    S: PlanningSolution,
    V: Clone + PartialEq + Send + Sync + Debug + 'static,
{
    type Undo = ();

    fn is_doable<D: Director<S>>(&self, score_director: &D) -> bool {
        multi_swap_is_doable(&self.access, &self.swaps, score_director)
    }

    fn do_move<D: Director<S>>(&self, score_director: &mut D) -> Self::Undo {
        multi_swap_do_move(&self.access, &self.swaps, &self.indices, score_director)
    }

    fn undo_move<D: Director<S>>(&self, score_director: &mut D, (): Self::Undo) {
        multi_swap_undo_move(&self.access, &self.swaps, &self.indices, score_director);
    }

    fn descriptor_index(&self) -> usize {
        self.access.descriptor_index
    }

    fn entity_indices(&self) -> &[usize] {
        &self.indices
    }

    fn variable_name(&self) -> &str {
        self.access.variable_name
    }

    fn telemetry_label(&self) -> &'static str {
        "list_multi_swap"
    }

    fn requires_score_improvement(&self) -> bool {
        self.require_score_improvement
    }

    fn tabu_signature<D: Director<S>>(&self, score_director: &D) -> MoveTabuSignature {
        multi_swap_tabu_signature(&self.access, &self.swaps, &self.indices, score_director)
    }
}
