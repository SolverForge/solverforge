use rand::rngs::SmallRng;
use rand::SeedableRng;
use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;

use crate::builder::context::{list_access::ListAccess, RuntimeListSlot};
use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::selector::list_kernel::{
    ChangeCursor, KOptCursor, NearbyChangeCursor, NearbyKOptCursor, NearbySwapCursor,
    PermuteCursor, PrecedenceCursor, ReverseCursor, RuinCursor, SublistChangeCursor,
    SublistSwapCursor, SwapCursor, DYNAMIC_CHANGE_SALTS, STATIC_CHANGE_SALTS,
    STATIC_NEARBY_CHANGE_ENTITY_SALT, STATIC_NEARBY_CHANGE_SOURCE_SALT,
    STATIC_NEARBY_SWAP_ENTITY_SALT, STATIC_NEARBY_SWAP_SOURCE_SALT, STATIC_REVERSE_ENTITY_SALT,
    STATIC_SUBLIST_CHANGE_SALTS, STATIC_SUBLIST_SWAP_ENTITY_SALT, STATIC_SWAP_SALTS,
};
use crate::heuristic::selector::move_selector::{
    CandidateId, MoveCandidateRef, MoveCursor, MoveStreamContext,
};
use crate::heuristic::selector::nearby_list_change::CrossEntityDistanceMeter;

use super::super::emission::RuntimeListEmitter;
use super::super::spec::RuntimeListNeighborhoodSpec;
use super::super::RuntimeListMove;
use super::probe::{
    runtime_precedence_analysis, runtime_precedence_graph, runtime_ruin_source_pool,
    runtime_selected_owners, RuntimeKOptProbe, RuntimeNearbyProbe,
};

pub(super) enum RuntimeListSlotCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    Change(ChangeCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    NearbyChange(
        NearbyChangeCursor<S, RuntimeListEmitter<S, V, DM, IDM>, RuntimeNearbyProbe<S, V, DM, IDM>>,
    ),
    Swap(SwapCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    Permute(PermuteCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    Precedence(PrecedenceCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    NearbySwap(
        NearbySwapCursor<S, RuntimeListEmitter<S, V, DM, IDM>, RuntimeNearbyProbe<S, V, DM, IDM>>,
    ),
    SublistChange(SublistChangeCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    SublistSwap(SublistSwapCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    Reverse(ReverseCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    KOpt(KOptCursor<'a, S, RuntimeListEmitter<S, V, DM, IDM>>),
    NearbyKOpt(
        NearbyKOptCursor<'a, S, RuntimeListEmitter<S, V, DM, IDM>, RuntimeKOptProbe<S, V, DM, IDM>>,
    ),
    Ruin(RuinCursor<S, RuntimeListEmitter<S, V, DM, IDM>>),
    Empty,
}

impl<S, V, DM, IDM> RuntimeListSlotCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    pub(super) fn next_move(&mut self) -> Option<RuntimeListMove<S, V, DM, IDM>> {
        match self {
            Self::Change(cursor) => cursor.next_owned_candidate(),
            Self::NearbyChange(cursor) => cursor.next_owned_candidate(),
            Self::Swap(cursor) => cursor.next_owned_candidate(),
            Self::Permute(cursor) => cursor.next_owned_candidate(),
            Self::Precedence(cursor) => cursor.next_owned_candidate(),
            Self::NearbySwap(cursor) => cursor.next_owned_candidate(),
            Self::SublistChange(cursor) => cursor.next_owned_candidate(),
            Self::SublistSwap(cursor) => cursor.next_owned_candidate(),
            Self::Reverse(cursor) => cursor.next_owned_candidate(),
            Self::KOpt(cursor) => cursor.next_owned_candidate(),
            Self::NearbyKOpt(cursor) => cursor.next_owned_candidate(),
            Self::Ruin(cursor) => cursor.next_owned_candidate(),
            Self::Empty => None,
        }
    }
}

impl<S, V, DM, IDM> MoveCursor<S, RuntimeListMove<S, V, DM, IDM>>
    for RuntimeListSlotCursor<'_, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    fn next_candidate(&mut self) -> Option<CandidateId> {
        match self {
            Self::Change(cursor) => cursor.next_candidate(),
            Self::NearbyChange(cursor) => cursor.next_candidate(),
            Self::Swap(cursor) => cursor.next_candidate(),
            Self::Permute(cursor) => cursor.next_candidate(),
            Self::Precedence(cursor) => cursor.next_candidate(),
            Self::NearbySwap(cursor) => cursor.next_candidate(),
            Self::SublistChange(cursor) => cursor.next_candidate(),
            Self::SublistSwap(cursor) => cursor.next_candidate(),
            Self::Reverse(cursor) => cursor.next_candidate(),
            Self::KOpt(cursor) => cursor.next_candidate(),
            Self::NearbyKOpt(cursor) => cursor.next_candidate(),
            Self::Ruin(cursor) => cursor.next_candidate(),
            Self::Empty => None,
        }
    }

    fn candidate(
        &self,
        id: CandidateId,
    ) -> Option<MoveCandidateRef<'_, S, RuntimeListMove<S, V, DM, IDM>>> {
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
            Self::Empty => None,
        }
    }

    fn take_candidate(&mut self, id: CandidateId) -> RuntimeListMove<S, V, DM, IDM> {
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
            Self::Empty => panic!("empty runtime list slot has no candidate to take"),
        }
    }

    fn next_owned_candidate(&mut self) -> Option<RuntimeListMove<S, V, DM, IDM>> {
        self.next_move()
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
            Self::Empty => false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn open_slot_cursor<'a, S, V, DM, IDM>(
    spec: RuntimeListNeighborhoodSpec,
    slot: RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    context: MoveStreamContext,
    kopt_patterns: &'a [KOptReconnection],
    ruin_seed: Option<u64>,
) -> RuntimeListSlotCursor<'a, S, V, DM, IDM>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let descriptor_index = slot.descriptor_index();
    let precedence_route_graph = runtime_precedence_graph(&slot, solution);
    // Only list-change has a public dynamic selector. Its
    // salt profile is preserved below. Dynamic bindings for the other list
    // families are newly exposed by this compiled leaf and intentionally use
    // the documented canonical shared-cursor profile, not a fallback path.
    match spec {
        RuntimeListNeighborhoodSpec::Change => {
            // The one shared cursor carries the dynamic change stream
            // profile as data. Every other operation remains a single common
            // implementation; this only preserves the already-public seeded
            // order of DynamicListChangeMoveSelector.
            let salts = match &slot {
                RuntimeListSlot::Static { .. } => STATIC_CHANGE_SALTS,
                RuntimeListSlot::Dynamic(_) => DYNAMIC_CHANGE_SALTS,
            };
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(salts.entity ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            RuntimeListSlotCursor::Change(
                ChangeCursor::new(
                    RuntimeListEmitter::new(slot, false),
                    entities,
                    route_lens,
                    context,
                    salts,
                    owners,
                    descriptor_index,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::NearbyChange { max_nearby } => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(STATIC_NEARBY_CHANGE_ENTITY_SALT ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            let entity_count = ListAccess::entity_count(&slot, solution);
            RuntimeListSlotCursor::NearbyChange(
                NearbyChangeCursor::new(
                    RuntimeListEmitter::new(slot.clone(), false),
                    (*solution).clone(),
                    RuntimeNearbyProbe::new(slot),
                    entities,
                    route_lens,
                    entity_count,
                    context,
                    owners.is_fixed_to_current(),
                    max_nearby,
                    descriptor_index,
                    STATIC_NEARBY_CHANGE_SOURCE_SALT,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::Swap => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(STATIC_SWAP_SALTS.entity ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            RuntimeListSlotCursor::Swap(
                SwapCursor::new(
                    RuntimeListEmitter::new(slot, false),
                    entities,
                    route_lens,
                    context,
                    STATIC_SWAP_SALTS,
                    owners,
                    descriptor_index,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::Permute {
            min_window_size,
            max_window_size,
        } => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(0x91D7_9E8A_0000_0001 ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            RuntimeListSlotCursor::Permute(
                PermuteCursor::new(
                    RuntimeListEmitter::new(slot, false),
                    entities,
                    route_lens,
                    context,
                    min_window_size,
                    max_window_size,
                    owners,
                    descriptor_index,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::Precedence => {
            let Some(analysis) =
                runtime_precedence_analysis(&slot, solution, precedence_route_graph)
            else {
                return RuntimeListSlotCursor::Empty;
            };
            RuntimeListSlotCursor::Precedence(PrecedenceCursor::new(
                analysis.blocks,
                analysis.route_graph,
                context,
                RuntimeListEmitter::new(slot, false),
                descriptor_index,
            ))
        }
        RuntimeListNeighborhoodSpec::NearbySwap { max_nearby } => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(STATIC_NEARBY_SWAP_ENTITY_SALT ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            let entity_count = ListAccess::entity_count(&slot, solution);
            RuntimeListSlotCursor::NearbySwap(
                NearbySwapCursor::new(
                    RuntimeListEmitter::new(slot.clone(), false),
                    (*solution).clone(),
                    RuntimeNearbyProbe::new(slot),
                    entities,
                    route_lens,
                    entity_count,
                    context,
                    owners.is_fixed_to_current(),
                    max_nearby,
                    descriptor_index,
                    STATIC_NEARBY_SWAP_SOURCE_SALT,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::SublistChange {
            min_sublist_size,
            max_sublist_size,
        } => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(STATIC_SUBLIST_CHANGE_SALTS.entity ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            RuntimeListSlotCursor::SublistChange(
                SublistChangeCursor::new(
                    RuntimeListEmitter::new(slot, false),
                    entities,
                    route_lens,
                    context,
                    STATIC_SUBLIST_CHANGE_SALTS,
                    min_sublist_size,
                    max_sublist_size,
                    owners,
                    descriptor_index,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::SublistSwap {
            min_sublist_size,
            max_sublist_size,
        } => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(STATIC_SUBLIST_SWAP_ENTITY_SALT ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(&slot, solution, &entities, &route_lens);
            RuntimeListSlotCursor::SublistSwap(
                SublistSwapCursor::new(
                    RuntimeListEmitter::new(slot, false),
                    entities,
                    route_lens,
                    context,
                    min_sublist_size,
                    max_sublist_size,
                    owners,
                    descriptor_index,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::Reverse => {
            let (entities, route_lens) = selected_entities(
                &slot,
                solution,
                context,
                Some(STATIC_REVERSE_ENTITY_SALT ^ descriptor_index as u64),
            );
            RuntimeListSlotCursor::Reverse(
                ReverseCursor::new(
                    RuntimeListEmitter::new(slot, false),
                    entities,
                    route_lens,
                    context,
                    descriptor_index,
                )
                .with_precedence_route_graph(precedence_route_graph),
            )
        }
        RuntimeListNeighborhoodSpec::KOpt {
            k,
            min_segment_len,
            max_nearby: 0,
        } => {
            let entities = (0..ListAccess::entity_count(&slot, solution))
                .map(|entity| (entity, ListAccess::list_len(&slot, solution, entity)))
                .collect();
            RuntimeListSlotCursor::KOpt(KOptCursor::new(
                RuntimeListEmitter::new(slot, false),
                entities,
                k,
                min_segment_len,
                kopt_patterns,
                context,
                descriptor_index,
            ))
        }
        RuntimeListNeighborhoodSpec::KOpt {
            k,
            min_segment_len,
            max_nearby,
        } => {
            let entities = (0..ListAccess::entity_count(&slot, solution)).collect();
            let length_slot = slot.clone();
            RuntimeListSlotCursor::NearbyKOpt(NearbyKOptCursor::new(
                RuntimeListEmitter::new(slot.clone(), false),
                (*solution).clone(),
                RuntimeKOptProbe::new(slot),
                entities,
                k,
                min_segment_len,
                max_nearby,
                kopt_patterns,
                move |solution, entity| ListAccess::list_len(&length_slot, solution, entity),
                context,
                descriptor_index,
            ))
        }
        RuntimeListNeighborhoodSpec::Ruin {
            min_ruin_count,
            max_ruin_count,
            moves_per_step,
            max_source_list_len,
            skip_empty_destinations,
        } => {
            let Some(seed) = ruin_seed else {
                return RuntimeListSlotCursor::Empty;
            };
            let source_pool = runtime_ruin_source_pool(&slot, solution, max_source_list_len);
            RuntimeListSlotCursor::Ruin(RuinCursor::new(
                RuntimeListEmitter::new(slot, skip_empty_destinations),
                SmallRng::seed_from_u64(seed),
                source_pool,
                moves_per_step,
                min_ruin_count,
                max_ruin_count,
            ))
        }
    }
}

fn selected_entities<S, V, DM, IDM>(
    slot: &RuntimeListSlot<S, V, DM, IDM>,
    solution: &S,
    context: MoveStreamContext,
    rotation_salt: Option<u64>,
) -> (Vec<usize>, Vec<usize>)
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    V: Clone + PartialEq + Send + Sync + std::fmt::Debug + 'static,
    DM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
    IDM: Clone + Send + Sync + std::fmt::Debug + CrossEntityDistanceMeter<S>,
{
    let canonical_entities = (0..ListAccess::entity_count(slot, solution)).collect::<Vec<_>>();
    let entities = match rotation_salt {
        Some(salt) => (0..canonical_entities.len())
            .map(|offset| {
                canonical_entities[context.selection_index_without_replacement(
                    offset,
                    canonical_entities.len(),
                    salt,
                )]
            })
            .collect(),
        None => canonical_entities,
    };
    let route_lens = entities
        .iter()
        .map(|&entity| ListAccess::list_len(slot, solution, entity))
        .collect();
    (entities, route_lens)
}
