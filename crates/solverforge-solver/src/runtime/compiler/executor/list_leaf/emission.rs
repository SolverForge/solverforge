use std::fmt;

use smallvec::SmallVec;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::builder::context::list_access::ListAccess;
use crate::builder::context::RuntimeListSlot;
use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::r#move::list_kernel::{
    merged_ruin_sources, single_ruin_source, ChangeCoordinates, MultiSwapCoordinates,
    PermuteCoordinates, ReverseCoordinates,
};
use crate::heuristic::r#move::metadata::hash_str;
use crate::heuristic::r#move::{CutPoint, MAX_LIST_PERMUTE_WINDOW_SIZE};
use crate::heuristic::r#move::{SegmentRelocationCoords, SegmentSwapCoords};
use crate::heuristic::selector::list_kernel::{
    ChangeEmitter, KOptEmitter, PermuteEmitter, PrecedenceEmitter, ReverseEmitter, RuinEmitter,
    SublistChangeEmitter, SublistSwapEmitter, SwapEmitter,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::move_access::RuntimeListMoveAccess;
use super::{RuntimeListMove, RuntimeListRecipe};

/// Recipe-only physical emitter for every shared list cursor.
///
/// It owns the selected slot but never performs enumeration, pruning, or
/// mutation. The cursor therefore remains the sole source of candidate order.
#[derive(Clone)]
pub(super) struct RuntimeListEmitter<S, V, DM, IDM> {
    slot: RuntimeListSlot<S, V, DM, IDM>,
    move_access: RuntimeListMoveAccess<S, V>,
    skip_empty_destinations: bool,
}

impl<S, V, DM, IDM> RuntimeListEmitter<S, V, DM, IDM>
where
    S: Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn new(slot: RuntimeListSlot<S, V, DM, IDM>, skip_empty_destinations: bool) -> Self {
        let move_access = RuntimeListMoveAccess::from_slot(&slot);
        Self {
            slot,
            move_access,
            skip_empty_destinations,
        }
    }

    fn move_from_recipe(&self, recipe: RuntimeListRecipe<S, V>) -> RuntimeListMove<S, V, DM, IDM> {
        RuntimeListMove::new(recipe)
    }
}

impl<S, V, DM, IDM> fmt::Debug for RuntimeListEmitter<S, V, DM, IDM>
where
    RuntimeListSlot<S, V, DM, IDM>: fmt::Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RuntimeListEmitter")
            .field("slot", &self.slot)
            .field("skip_empty_destinations", &self.skip_empty_destinations)
            .finish()
    }
}

impl<S, V, DM, IDM> ChangeEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_change(
        &self,
        source_entity: usize,
        source_position: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::Change {
            access: self.move_access.clone(),
            coordinates: ChangeCoordinates {
                source_entity,
                source_position,
                destination_entity,
                destination_position,
            },
        })
    }
}

impl<S, V, DM, IDM> SwapEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_swap(
        &self,
        first_entity: usize,
        first_position: usize,
        second_entity: usize,
        second_position: usize,
    ) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::Swap {
            access: self.move_access.clone(),
            coordinates: crate::heuristic::r#move::list_kernel::SwapCoordinates {
                first_entity,
                first_position,
                second_entity,
                second_position,
            },
        })
    }
}

impl<S, V, DM, IDM> ReverseEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_reverse(&self, entity: usize, start: usize, end: usize) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::Reverse {
            access: self.move_access.clone(),
            coordinates: ReverseCoordinates { entity, start, end },
        })
    }
}

impl<S, V, DM, IDM> PermuteEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_permute(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::Permute {
            access: self.move_access.clone(),
            coordinates: PermuteCoordinates { entity, start, end },
            inverse_permutation: inverse_permutation(&permutation),
            permutation,
        })
    }
}

impl<S, V, DM, IDM> SublistChangeEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_sublist_change(
        &self,
        source_entity: usize,
        source_start: usize,
        source_end: usize,
        destination_entity: usize,
        destination_position: usize,
    ) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::SublistChange {
            access: self.move_access.clone(),
            coordinates: SegmentRelocationCoords::new(
                source_entity,
                source_start,
                source_end,
                destination_entity,
                destination_position,
            ),
        })
    }
}

impl<S, V, DM, IDM> SublistSwapEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_sublist_swap(
        &self,
        first_entity: usize,
        first_start: usize,
        first_end: usize,
        second_entity: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::SublistSwap {
            access: self.move_access.clone(),
            coordinates: SegmentSwapCoords::new(
                first_entity,
                first_start,
                first_end,
                second_entity,
                second_start,
                second_end,
            ),
        })
    }
}

impl<S, V, DM, IDM> KOptEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_k_opt(&self, cuts: &[CutPoint], reconnection: &KOptReconnection) -> Self::Move {
        let entity = cuts
            .first()
            .map(CutPoint::entity_index)
            .expect("shared K-opt cursor emits at least one cut");
        self.move_from_recipe(RuntimeListRecipe::KOpt {
            access: self.move_access.clone(),
            entity,
            cuts: cuts.iter().copied().collect(),
            reconnection: *reconnection,
            variable_id: hash_str(self.slot.variable_name()),
        })
    }
}

impl<S, V, DM, IDM> RuinEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_ruin(&self, entity: usize, indices: &[usize]) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::Ruin {
            access: self.move_access.clone(),
            sources: single_ruin_source(entity, indices),
            skip_empty_destinations: self.skip_empty_destinations,
        })
    }
}

impl<S, V, DM, IDM> PrecedenceEmitter<S> for RuntimeListEmitter<S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
    DM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + fmt::Debug + CrossEntityDistanceMeter<S>,
{
    type Move = RuntimeListMove<S, V, DM, IDM>;

    fn emit_change(&self, entity: usize, source: usize, destination: usize) -> Self::Move {
        <Self as ChangeEmitter<S>>::emit_change(self, entity, source, entity, destination)
    }

    fn emit_swap(&self, entity: usize, first: usize, second: usize) -> Self::Move {
        <Self as SwapEmitter<S>>::emit_swap(self, entity, first, entity, second)
    }

    fn emit_reverse(&self, entity: usize, start: usize, end: usize) -> Self::Move {
        <Self as ReverseEmitter<S>>::emit_reverse(self, entity, start, end)
    }

    fn emit_sublist_swap(
        &self,
        entity: usize,
        first_start: usize,
        first_end: usize,
        second_start: usize,
        second_end: usize,
    ) -> Self::Move {
        <Self as SublistSwapEmitter<S>>::emit_sublist_swap(
            self,
            entity,
            first_start,
            first_end,
            entity,
            second_start,
            second_end,
        )
    }

    fn emit_ruin(&self, sources: &[(usize, SmallVec<[usize; 8]>)]) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::Ruin {
            access: self.move_access.clone(),
            sources: merged_ruin_sources(sources),
            skip_empty_destinations: false,
        })
    }

    fn emit_sublist_change(
        &self,
        entity: usize,
        source_start: usize,
        source_end: usize,
        destination: usize,
    ) -> Self::Move {
        <Self as SublistChangeEmitter<S>>::emit_sublist_change(
            self,
            entity,
            source_start,
            source_end,
            entity,
            destination,
        )
    }

    fn emit_permute(
        &self,
        entity: usize,
        start: usize,
        end: usize,
        permutation: SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]>,
    ) -> Self::Move {
        <Self as PermuteEmitter<S>>::emit_permute(self, entity, start, end, permutation)
    }

    fn emit_multi_swap(&self, swaps: &[(usize, usize, usize)]) -> Self::Move {
        self.move_from_recipe(RuntimeListRecipe::MultiSwap {
            access: self.move_access.clone(),
            coordinates: MultiSwapCoordinates::from_slice(swaps),
            require_score_improvement: true,
        })
    }
}

fn inverse_permutation(permutation: &[usize]) -> SmallVec<[usize; MAX_LIST_PERMUTE_WINDOW_SIZE]> {
    let mut inverse = SmallVec::from_elem(0, permutation.len());
    for (position, &source) in permutation.iter().enumerate() {
        inverse[source] = position;
    }
    inverse
}
