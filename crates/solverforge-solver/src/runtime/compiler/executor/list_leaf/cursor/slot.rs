use rand::rngs::SmallRng;
use rand::SeedableRng;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::builder::context::{list_access::ListAccess, RuntimeListSlot};
use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::selector::list_kernel::{
    ChangeCursor, ContiguousRuinCursor, GroupChangeCursor, GroupSwapCursor, KOptCursor,
    NearbyChangeCursor, NearbyKOptCursor, NearbySwapCursor, PermuteCursor, PrecedenceCursor,
    ReverseCursor, RuinCursor, SublistChangeCursor, SublistSwapCursor, SwapCursor,
    DYNAMIC_CHANGE_SALTS, STATIC_CHANGE_SALTS, STATIC_NEARBY_CHANGE_ENTITY_SALT,
    STATIC_NEARBY_CHANGE_SOURCE_SALT, STATIC_NEARBY_SWAP_ENTITY_SALT,
    STATIC_NEARBY_SWAP_SOURCE_SALT, STATIC_REVERSE_ENTITY_SALT, STATIC_SUBLIST_CHANGE_SALTS,
    STATIC_SUBLIST_SWAP_ENTITY_SALT, STATIC_SWAP_SALTS,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::emission::RuntimeListEmitter;
use super::super::group_emission::GroupedRuinEmitter;
use super::super::spec::{RuinSourceShape, RuntimeListNeighborhoodSpec};
use super::super::RuntimeListMove;
use super::groups::{runtime_group_snapshots, runtime_grouped_ruin_emitter};
use super::probe::{
    runtime_precedence_analysis, runtime_precedence_graph, runtime_ruin_source_pool,
    runtime_selected_owners, RuntimeKOptProbe, RuntimeNearbyProbe, RuntimeRuinProbe,
};

mod open;
pub(super) use open::open_slot_cursor;

pub(super) enum RuntimeListSlotCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    Change(ChangeCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    NearbyChange(
        NearbyChangeCursor<
            S,
            RuntimeListEmitter<'a, S, V, DM, IDM>,
            RuntimeNearbyProbe<'a, S, V, DM, IDM>,
        >,
    ),
    Swap(SwapCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    Permute(PermuteCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    Precedence(PrecedenceCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    NearbySwap(
        NearbySwapCursor<
            S,
            RuntimeListEmitter<'a, S, V, DM, IDM>,
            RuntimeNearbyProbe<'a, S, V, DM, IDM>,
        >,
    ),
    SublistChange(SublistChangeCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    SublistSwap(SublistSwapCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    Reverse(ReverseCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    KOpt(KOptCursor<'a, S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    NearbyKOpt(
        NearbyKOptCursor<
            'a,
            S,
            RuntimeListEmitter<'a, S, V, DM, IDM>,
            RuntimeKOptProbe<'a, S, V, DM, IDM>,
        >,
    ),
    Ruin(RuinCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    GroupRuin(RuinCursor<S, GroupedRuinEmitter<'a, S, V, DM, IDM>>),
    ContiguousRuin(
        ContiguousRuinCursor<
            S,
            RuntimeListEmitter<'a, S, V, DM, IDM>,
            RuntimeRuinProbe<'a, S, V, DM, IDM>,
        >,
    ),
    ContiguousGroupRuin(
        ContiguousRuinCursor<
            S,
            GroupedRuinEmitter<'a, S, V, DM, IDM>,
            RuntimeRuinProbe<'a, S, V, DM, IDM>,
        >,
    ),
    GroupChange(GroupChangeCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    GroupSwap(GroupSwapCursor<S, RuntimeListEmitter<'a, S, V, DM, IDM>>),
    Empty,
}

impl<'a, S, V, DM, IDM> RuntimeListSlotCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn next_move(&mut self, solution: &S) -> Option<RuntimeListMove<'a, S, V, DM, IDM>> {
        match self {
            Self::Change(cursor) => cursor.next_owned_candidate(solution),
            Self::NearbyChange(cursor) => cursor.next_owned_candidate(solution),
            Self::Swap(cursor) => cursor.next_owned_candidate(solution),
            Self::Permute(cursor) => cursor.next_owned_candidate(solution),
            Self::Precedence(cursor) => cursor.next_owned_candidate(solution),
            Self::NearbySwap(cursor) => cursor.next_owned_candidate(solution),
            Self::SublistChange(cursor) => cursor.next_owned_candidate(solution),
            Self::SublistSwap(cursor) => cursor.next_owned_candidate(solution),
            Self::Reverse(cursor) => cursor.next_owned_candidate(solution),
            Self::KOpt(cursor) => cursor.next_owned_candidate(solution),
            Self::NearbyKOpt(cursor) => cursor.next_owned_candidate(solution),
            Self::Ruin(cursor) => cursor.next_owned_candidate(solution),
            Self::GroupRuin(cursor) => cursor.next_owned_candidate(solution),
            Self::ContiguousRuin(cursor) => cursor.next_owned_candidate(solution),
            Self::ContiguousGroupRuin(cursor) => cursor.next_owned_candidate(solution),
            Self::GroupChange(cursor) => cursor.next_owned_candidate(solution),
            Self::GroupSwap(cursor) => cursor.next_owned_candidate(solution),
            Self::Empty => None,
        }
    }
}

impl<'a, S, V, DM, IDM> MoveCursor<S, RuntimeListMove<'a, S, V, DM, IDM>>
    for RuntimeListSlotCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn next_candidate(&mut self, solution: &S) -> Option<CandidateId> {
        match self {
            Self::Change(cursor) => cursor.next_candidate(solution),
            Self::NearbyChange(cursor) => cursor.next_candidate(solution),
            Self::Swap(cursor) => cursor.next_candidate(solution),
            Self::Permute(cursor) => cursor.next_candidate(solution),
            Self::Precedence(cursor) => cursor.next_candidate(solution),
            Self::NearbySwap(cursor) => cursor.next_candidate(solution),
            Self::SublistChange(cursor) => cursor.next_candidate(solution),
            Self::SublistSwap(cursor) => cursor.next_candidate(solution),
            Self::Reverse(cursor) => cursor.next_candidate(solution),
            Self::KOpt(cursor) => cursor.next_candidate(solution),
            Self::NearbyKOpt(cursor) => cursor.next_candidate(solution),
            Self::Ruin(cursor) => cursor.next_candidate(solution),
            Self::GroupRuin(cursor) => cursor.next_candidate(solution),
            Self::ContiguousRuin(cursor) => cursor.next_candidate(solution),
            Self::ContiguousGroupRuin(cursor) => cursor.next_candidate(solution),
            Self::GroupChange(cursor) => cursor.next_candidate(solution),
            Self::GroupSwap(cursor) => cursor.next_candidate(solution),
            Self::Empty => None,
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, RuntimeListMove<'a, S, V, DM, IDM>>> {
        match self {
            Self::Change(cursor) => cursor.candidate(id),
            Self::NearbyChange(cursor) => cursor.candidate(id),
            Self::Swap(cursor) => cursor.candidate(id),
            Self::Permute(cursor) => cursor.candidate(id),
            Self::Precedence(cursor) => cursor.candidate(id),
            Self::NearbySwap(cursor) => cursor.candidate(id),
            Self::SublistChange(cursor) => cursor.candidate(id),
            Self::SublistSwap(cursor) => cursor.candidate(id),
            Self::Reverse(cursor) => cursor.candidate(id),
            Self::KOpt(cursor) => cursor.candidate(id),
            Self::NearbyKOpt(cursor) => cursor.candidate(id),
            Self::Ruin(cursor) => cursor.candidate(id),
            Self::GroupRuin(cursor) => cursor.candidate(id),
            Self::ContiguousRuin(cursor) => cursor.candidate(id),
            Self::ContiguousGroupRuin(cursor) => cursor.candidate(id),
            Self::GroupChange(cursor) => cursor.candidate(id),
            Self::GroupSwap(cursor) => cursor.candidate(id),
            Self::Empty => None,
        }
    }

    fn take_candidate(&mut self, id: CandidateId) -> RuntimeListMove<'a, S, V, DM, IDM> {
        match self {
            Self::Change(cursor) => cursor.take_candidate(id),
            Self::NearbyChange(cursor) => cursor.take_candidate(id),
            Self::Swap(cursor) => cursor.take_candidate(id),
            Self::Permute(cursor) => cursor.take_candidate(id),
            Self::Precedence(cursor) => cursor.take_candidate(id),
            Self::NearbySwap(cursor) => cursor.take_candidate(id),
            Self::SublistChange(cursor) => cursor.take_candidate(id),
            Self::SublistSwap(cursor) => cursor.take_candidate(id),
            Self::Reverse(cursor) => cursor.take_candidate(id),
            Self::KOpt(cursor) => cursor.take_candidate(id),
            Self::NearbyKOpt(cursor) => cursor.take_candidate(id),
            Self::Ruin(cursor) => cursor.take_candidate(id),
            Self::GroupRuin(cursor) => cursor.take_candidate(id),
            Self::ContiguousRuin(cursor) => cursor.take_candidate(id),
            Self::ContiguousGroupRuin(cursor) => cursor.take_candidate(id),
            Self::GroupChange(cursor) => cursor.take_candidate(id),
            Self::GroupSwap(cursor) => cursor.take_candidate(id),
            Self::Empty => panic!("empty runtime list slot has no candidate to take"),
        }
    }

    fn next_owned_candidate(&mut self, solution: &S) -> Option<RuntimeListMove<'a, S, V, DM, IDM>> {
        self.next_move(solution)
    }

    fn release_candidate(&mut self, id: CandidateId) -> bool {
        match self {
            Self::Change(cursor) => cursor.release_candidate(id),
            Self::NearbyChange(cursor) => cursor.release_candidate(id),
            Self::Swap(cursor) => cursor.release_candidate(id),
            Self::Permute(cursor) => cursor.release_candidate(id),
            Self::Precedence(cursor) => cursor.release_candidate(id),
            Self::NearbySwap(cursor) => cursor.release_candidate(id),
            Self::SublistChange(cursor) => cursor.release_candidate(id),
            Self::SublistSwap(cursor) => cursor.release_candidate(id),
            Self::Reverse(cursor) => cursor.release_candidate(id),
            Self::KOpt(cursor) => cursor.release_candidate(id),
            Self::NearbyKOpt(cursor) => cursor.release_candidate(id),
            Self::Ruin(cursor) => cursor.release_candidate(id),
            Self::GroupRuin(cursor) => cursor.release_candidate(id),
            Self::ContiguousRuin(cursor) => cursor.release_candidate(id),
            Self::ContiguousGroupRuin(cursor) => cursor.release_candidate(id),
            Self::GroupChange(cursor) => cursor.release_candidate(id),
            Self::GroupSwap(cursor) => cursor.release_candidate(id),
            Self::Empty => false,
        }
    }
}
