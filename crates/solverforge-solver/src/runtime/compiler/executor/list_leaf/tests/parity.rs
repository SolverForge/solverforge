use crate::heuristic::r#move::k_opt_reconnection::KOptReconnection;
use crate::heuristic::r#move::Move;
use crate::heuristic::selector::dynamic_list_change::DynamicListChangeMoveSelector;
use crate::heuristic::selector::entity::FromSolutionEntitySelector;
use crate::heuristic::selector::list_change::ListChangeMoveSelector;
use crate::heuristic::selector::list_permute::ListPermuteMoveSelector;
use crate::heuristic::selector::move_selector::{MoveCursor, MoveSelector, MoveStreamContext};

use super::super::super::super::graph::ListLeafKind;
use super::super::spec::RuntimeListRecipe;
use super::support::{
    config, director, dynamic_slot, dynamic_slot_for, dynamic_variable_slot, initial_plan,
    list_get, list_len, native_slot, selector, static_slot, static_slot_for, ListPlan, ALL_KINDS,
};

#[derive(Debug, PartialEq, Eq)]
enum RecipeSignature {
    Change(usize, usize, usize, usize),
    Swap(usize, usize, usize, usize),
    Permute(usize, usize, usize, Vec<usize>),
    Reverse(usize, usize, usize),
    SublistChange(usize, usize, usize, usize, usize),
    SublistSwap(usize, usize, usize, usize, usize, usize),
    KOpt(usize, Vec<(usize, usize)>, KOptReconnection),
    Ruin(Vec<(usize, Vec<usize>)>),
    MultiSwap(Vec<(usize, usize, usize)>),
}

fn signatures(
    kind: ListLeafKind,
    selector: super::super::RuntimeListNeighborhoodSelector<
        ListPlan,
        usize,
        super::support::PositionMetric,
        super::support::PositionMetric,
    >,
    context: MoveStreamContext,
) -> Vec<RecipeSignature> {
    let score_director = director(initial_plan());
    let mut stream_state = selector.new_stream_state();
    let mut cursor =
        selector.open_cursor_with_stream_state(&mut stream_state, &score_director, context);
    let mut signatures = Vec::new();
    while let Some(move_) = cursor.next_owned_candidate() {
        let signature = match move_.into_recipe() {
            RuntimeListRecipe::Change { coordinates, .. } => RecipeSignature::Change(
                coordinates.source_entity,
                coordinates.source_position,
                coordinates.destination_entity,
                coordinates.destination_position,
            ),
            RuntimeListRecipe::Swap { coordinates, .. } => RecipeSignature::Swap(
                coordinates.first_entity,
                coordinates.first_position,
                coordinates.second_entity,
                coordinates.second_position,
            ),
            RuntimeListRecipe::Permute {
                coordinates,
                permutation,
                ..
            } => RecipeSignature::Permute(
                coordinates.entity,
                coordinates.start,
                coordinates.end,
                permutation.into_vec(),
            ),
            RuntimeListRecipe::Reverse { coordinates, .. } => {
                RecipeSignature::Reverse(coordinates.entity, coordinates.start, coordinates.end)
            }
            RuntimeListRecipe::SublistChange { coordinates, .. } => RecipeSignature::SublistChange(
                coordinates.source_entity_index,
                coordinates.source_range.start,
                coordinates.source_range.end,
                coordinates.dest_entity_index,
                coordinates.dest_position,
            ),
            RuntimeListRecipe::SublistSwap { coordinates, .. } => RecipeSignature::SublistSwap(
                coordinates.first_entity_index,
                coordinates.first_range.start,
                coordinates.first_range.end,
                coordinates.second_entity_index,
                coordinates.second_range.start,
                coordinates.second_range.end,
            ),
            RuntimeListRecipe::KOpt {
                entity,
                cuts,
                reconnection,
                ..
            } => RecipeSignature::KOpt(
                entity,
                cuts.into_iter()
                    .map(|cut| (cut.entity_index(), cut.position()))
                    .collect(),
                reconnection,
            ),
            RuntimeListRecipe::Ruin { sources, .. } => RecipeSignature::Ruin(
                sources
                    .into_iter()
                    .map(|(entity, positions)| (entity, positions.into_vec()))
                    .collect(),
            ),
            RuntimeListRecipe::MultiSwap { coordinates, .. } => {
                RecipeSignature::MultiSwap(coordinates.into_vec())
            }
        };
        signatures.push(signature);
    }
    assert!(matches!(kind, ListLeafKind::Change | ListLeafKind::Ruin) || !signatures.is_empty());
    signatures
}

fn change_signature(
    source_entity: usize,
    source_position: usize,
    destination_entity: usize,
    destination_position: usize,
) -> RecipeSignature {
    RecipeSignature::Change(
        source_entity,
        source_position,
        destination_entity,
        destination_position,
    )
}

fn native_change_signatures(context: MoveStreamContext) -> Vec<RecipeSignature> {
    let score_director = director(initial_plan());
    let slot = native_slot();
    ListChangeMoveSelector::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        slot.list_remove,
        slot.list_insert,
        "visits",
        0,
    )
    .open_cursor_with_context(&score_director, context)
    .map(|move_| {
        change_signature(
            move_.source_entity_index(),
            move_.source_position(),
            move_.dest_entity_index(),
            move_.dest_position(),
        )
    })
    .collect()
}

fn direct_dynamic_change_signatures(context: MoveStreamContext) -> Vec<RecipeSignature> {
    let score_director = director(initial_plan());
    DynamicListChangeMoveSelector::new(dynamic_variable_slot())
        .open_cursor_with_context(&score_director, context)
        .map(|move_| {
            change_signature(
                move_.source_entity_index(),
                move_.source_position(),
                move_.dest_entity_index(),
                move_.dest_position(),
            )
        })
        .collect()
}

#[test]
fn every_compiled_list_family_preserves_typed_dynamic_recipe_order() {
    let context = MoveStreamContext::new(7, 41, Some(256));
    for kind in ALL_KINDS {
        let typed = signatures(
            kind,
            selector(kind, static_slot_for(kind), Some(97)),
            context,
        );
        let dynamic = signatures(
            kind,
            selector(kind, dynamic_slot_for(kind), Some(97)),
            context,
        );
        assert!(!typed.is_empty(), "{kind:?} must yield test recipes");
        if kind != ListLeafKind::Change {
            assert_eq!(
                typed, dynamic,
                "{kind:?} canonical typed/dynamic recipe sequence"
            );
        }
    }
}

#[test]
fn change_keeps_each_public_carrier_seeded_order_profile() {
    let context = MoveStreamContext::new(5, 89, Some(64));
    let static_runtime = signatures(
        ListLeafKind::Change,
        selector(ListLeafKind::Change, static_slot(), Some(97)),
        context,
    );
    let dynamic_runtime = signatures(
        ListLeafKind::Change,
        selector(ListLeafKind::Change, dynamic_slot(), Some(97)),
        context,
    );
    assert_eq!(static_runtime, native_change_signatures(context));
    assert_eq!(dynamic_runtime, direct_dynamic_change_signatures(context));
}

#[test]
fn change_carrier_profiles_keep_candidate_trace_identity() {
    let context = MoveStreamContext::new(5, 89, Some(64));
    let static_director = director(initial_plan());
    let native_slot = native_slot();
    let static_native = ListChangeMoveSelector::new(
        FromSolutionEntitySelector::new(0),
        list_len,
        list_get,
        native_slot.list_remove,
        native_slot.list_insert,
        "visits",
        0,
    )
    .open_cursor_with_context(&static_director, context)
    .next_owned_candidate()
    .expect("native static change emits a candidate");
    let static_runtime_selector = selector(ListLeafKind::Change, static_slot(), Some(97));
    let static_runtime = static_runtime_selector
        .open_cursor_with_context(&static_director, context)
        .next_owned_candidate()
        .expect("runtime static change emits a candidate");
    assert_eq!(
        static_runtime.candidate_trace_identity(),
        static_native.candidate_trace_identity()
    );

    let dynamic_director = director(initial_plan());
    let dynamic_native = DynamicListChangeMoveSelector::new(dynamic_variable_slot())
        .open_cursor_with_context(&dynamic_director, context)
        .next_owned_candidate()
        .expect("direct dynamic change emits a candidate");
    let dynamic_runtime_selector = selector(ListLeafKind::Change, dynamic_slot(), Some(97));
    let dynamic_runtime = dynamic_runtime_selector
        .open_cursor_with_context(&dynamic_director, context)
        .next_owned_candidate()
        .expect("runtime dynamic change emits a candidate");
    assert_eq!(
        dynamic_runtime.candidate_trace_identity(),
        dynamic_native.candidate_trace_identity()
    );
}

#[test]
fn seeded_runtime_ruin_batches_are_reproducible_without_a_second_enumerator() {
    let context = MoveStreamContext::new(3, 99, Some(10));
    let first = signatures(
        ListLeafKind::Ruin,
        selector(ListLeafKind::Ruin, static_slot(), Some(1_337)),
        context,
    );
    let second = signatures(
        ListLeafKind::Ruin,
        selector(ListLeafKind::Ruin, static_slot(), Some(1_337)),
        context,
    );
    assert_eq!(first, second);
    assert_eq!(first.len(), 3);
}

#[test]
#[should_panic(
    expected = "runtime list ruin leaves must be opened through their persistent composed stream state"
)]
fn direct_list_ruin_open_cannot_reset_the_per_solve_stream() {
    let selector = selector(ListLeafKind::Ruin, static_slot(), Some(1_337));
    let score_director = director(initial_plan());
    let _ = selector.open_cursor(&score_director);
}

#[test]
fn permute_keeps_native_entity_order_for_noncanonical_context() {
    let context = MoveStreamContext::new(23, 71, Some(64));
    let score_director = director(initial_plan());
    let slot = native_slot();
    let native = ListPermuteMoveSelector::new(
        FromSolutionEntitySelector::new(0),
        2,
        3,
        list_len,
        list_get,
        slot.sublist_remove,
        slot.sublist_insert,
        "visits",
        0,
    )
    .open_cursor_with_context(&score_director, context)
    .map(|move_| {
        RecipeSignature::Permute(
            move_.entity_index(),
            move_.start(),
            move_.end(),
            move_.permutation().to_vec(),
        )
    })
    .collect::<Vec<_>>();
    let runtime = signatures(
        ListLeafKind::Permute,
        selector(ListLeafKind::Permute, static_slot(), Some(97)),
        context,
    );
    assert_eq!(runtime, native);
    assert!(matches!(
        config(ListLeafKind::Permute),
        solverforge_config::MoveSelectorConfig::ListPermuteMoveSelector(_)
    ));
}
