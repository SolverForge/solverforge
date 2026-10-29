use super::*;

#[allow(clippy::too_many_arguments)]
pub(in super::super) fn open_slot_cursor<'a, S, V, DM, IDM>(
    spec: RuntimeListNeighborhoodSpec,
    slot: &'a RuntimeListSlot<S, V, DM, IDM>,
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
    let precedence_route_graph = runtime_precedence_graph(slot, solution);
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
            let salts = match slot {
                RuntimeListSlot::Static { .. } => STATIC_CHANGE_SALTS,
                RuntimeListSlot::Dynamic(_) => DYNAMIC_CHANGE_SALTS,
            };
            let (entities, route_lens) = selected_entities(
                slot,
                solution,
                context,
                Some(salts.entity ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
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
                slot,
                solution,
                context,
                Some(STATIC_NEARBY_CHANGE_ENTITY_SALT ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
            let entity_count = ListAccess::entity_count(slot, solution);
            RuntimeListSlotCursor::NearbyChange(
                NearbyChangeCursor::new(
                    RuntimeListEmitter::new(slot, false),
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
                slot,
                solution,
                context,
                Some(STATIC_SWAP_SALTS.entity ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
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
                slot,
                solution,
                context,
                Some(0x91D7_9E8A_0000_0001 ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
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
                runtime_precedence_analysis(slot, solution, precedence_route_graph)
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
                slot,
                solution,
                context,
                Some(STATIC_NEARBY_SWAP_ENTITY_SALT ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
            let entity_count = ListAccess::entity_count(slot, solution);
            RuntimeListSlotCursor::NearbySwap(
                NearbySwapCursor::new(
                    RuntimeListEmitter::new(slot, false),
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
                slot,
                solution,
                context,
                Some(STATIC_SUBLIST_CHANGE_SALTS.entity ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
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
                slot,
                solution,
                context,
                Some(STATIC_SUBLIST_SWAP_ENTITY_SALT ^ descriptor_index as u64),
            );
            let owners = runtime_selected_owners(slot, solution, &entities, &route_lens);
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
                slot,
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
            let entities = (0..ListAccess::entity_count(slot, solution))
                .map(|entity| (entity, ListAccess::list_len(slot, solution, entity)))
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
            let entities = (0..ListAccess::entity_count(slot, solution))
                .map(|entity| (entity, ListAccess::list_len(slot, solution, entity)))
                .collect();
            RuntimeListSlotCursor::NearbyKOpt(NearbyKOptCursor::new(
                RuntimeListEmitter::new(slot, false),
                RuntimeKOptProbe::new(slot),
                entities,
                k,
                min_segment_len,
                max_nearby,
                kopt_patterns,
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
            let source_pool = runtime_ruin_source_pool(slot, solution, max_source_list_len);
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
