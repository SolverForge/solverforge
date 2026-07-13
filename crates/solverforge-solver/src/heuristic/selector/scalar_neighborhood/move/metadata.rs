use smallvec::{smallvec, SmallVec};
use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::Director;

use crate::builder::RuntimeScalarSlot;
use crate::heuristic::r#move::metadata::{
    append_canonical_usize_slice_pair, encode_option_debug, encode_option_usize, encode_usize,
    hash_str, ordered_coordinate_pair, scoped_move_identity, MoveTabuScope, TABU_OP_PILLAR_SWAP,
    TABU_OP_SWAP,
};
use crate::heuristic::r#move::MoveTabuSignature;

use super::super::spec::RuntimeScalarRecipe;
use super::recipe_slot;

pub(super) fn tabu_signature<S, D>(
    recipe: &RuntimeScalarRecipe<S>,
    director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    D: Director<S>,
{
    let slot = recipe_slot(recipe);
    let scope = MoveTabuScope::new(slot.descriptor_index(), slot.variable_name());
    match recipe {
        RuntimeScalarRecipe::Change {
            entity_index,
            to_value,
            ..
        } => change(scope, slot, director, *entity_index, *to_value),
        RuntimeScalarRecipe::Swap {
            left_entity_index,
            right_entity_index,
            ..
        } => swap(
            scope,
            slot,
            director,
            *left_entity_index,
            *right_entity_index,
        ),
        RuntimeScalarRecipe::PillarChange {
            entity_indices,
            to_value,
            ..
        } => pillar_change(scope, slot, director, entity_indices, *to_value),
        RuntimeScalarRecipe::PillarSwap {
            left_indices,
            right_indices,
            ..
        } => pillar_swap(scope, slot, director, left_indices, right_indices),
        RuntimeScalarRecipe::RuinRecreate {
            entity_indices,
            recreate_heuristic_type,
            ..
        } => ruin(
            scope,
            slot,
            director,
            entity_indices,
            *recreate_heuristic_type,
        ),
    }
}

fn change<S, D>(
    scope: MoveTabuScope,
    slot: &RuntimeScalarSlot<S>,
    director: &D,
    entity_index: usize,
    to_value: Option<usize>,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    D: Director<S>,
{
    let current = slot.current_value(director.working_solution(), entity_index);
    let from = encode_option_debug(current.as_ref());
    let to = encode_option_debug(to_value.as_ref());
    let entity = encode_usize(entity_index);
    let variable = hash_str(slot.variable_name());
    MoveTabuSignature::new(
        scope,
        smallvec![
            encode_usize(slot.descriptor_index()),
            variable,
            entity,
            from,
            to
        ],
        smallvec![
            encode_usize(slot.descriptor_index()),
            variable,
            entity,
            to,
            from
        ],
    )
    .with_entity_tokens([scope.entity_token(entity)])
    .with_destination_value_tokens([scope.value_token(to)])
}

fn swap<S, D>(
    scope: MoveTabuScope,
    slot: &RuntimeScalarSlot<S>,
    director: &D,
    left: usize,
    right: usize,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    D: Director<S>,
{
    let left_current = slot.current_value(director.working_solution(), left);
    let right_current = slot.current_value(director.working_solution(), right);
    let left_value = encode_option_debug(left_current.as_ref());
    let right_value = encode_option_debug(right_current.as_ref());
    let entities = ordered_coordinate_pair((encode_usize(left), 0), (encode_usize(right), 0));
    let move_id = scoped_move_identity(
        scope,
        TABU_OP_SWAP,
        entities.into_iter().map(|(entity, _)| entity),
    );
    MoveTabuSignature::new(scope, move_id.clone(), move_id)
        .with_entity_tokens([
            scope.entity_token(encode_usize(left)),
            scope.entity_token(encode_usize(right)),
        ])
        .with_destination_value_tokens([
            scope.value_token(right_value),
            scope.value_token(left_value),
        ])
}

fn pillar_change<S, D>(
    scope: MoveTabuScope,
    slot: &RuntimeScalarSlot<S>,
    director: &D,
    entities: &[usize],
    to_value: Option<usize>,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    D: Director<S>,
{
    let from = entities
        .first()
        .and_then(|&entity| slot.current_value(director.working_solution(), entity));
    let from = encode_option_debug(from.as_ref());
    let to = encode_option_debug(to_value.as_ref());
    let variable = hash_str(slot.variable_name());
    let entity_ids = entities
        .iter()
        .copied()
        .map(encode_usize)
        .collect::<SmallVec<[_; 8]>>();
    let mut move_id = smallvec![
        encode_usize(slot.descriptor_index()),
        variable,
        encode_usize(entities.len()),
        from,
        to
    ];
    move_id.extend(entity_ids.iter().copied());
    let mut undo_move_id = smallvec![
        encode_usize(slot.descriptor_index()),
        variable,
        encode_usize(entities.len()),
        to,
        from
    ];
    undo_move_id.extend(entity_ids.iter().copied());
    MoveTabuSignature::new(scope, move_id, undo_move_id)
        .with_entity_tokens(
            entity_ids
                .iter()
                .copied()
                .map(|entity| scope.entity_token(entity)),
        )
        .with_destination_value_tokens([scope.value_token(to)])
}

fn pillar_swap<S, D>(
    scope: MoveTabuScope,
    slot: &RuntimeScalarSlot<S>,
    director: &D,
    left: &[usize],
    right: &[usize],
) -> MoveTabuSignature
where
    S: PlanningSolution,
    D: Director<S>,
{
    let left_value = left
        .first()
        .and_then(|&entity| slot.current_value(director.working_solution(), entity));
    let right_value = right
        .first()
        .and_then(|&entity| slot.current_value(director.working_solution(), entity));
    let mut move_id = scoped_move_identity(scope, TABU_OP_PILLAR_SWAP, std::iter::empty());
    append_canonical_usize_slice_pair(&mut move_id, left, right);
    let mut entities = left
        .iter()
        .chain(right)
        .copied()
        .map(encode_usize)
        .collect::<Vec<_>>();
    entities.sort_unstable();
    entities.dedup();
    MoveTabuSignature::new(scope, move_id.clone(), move_id)
        .with_entity_tokens(
            entities
                .iter()
                .copied()
                .map(|entity| scope.entity_token(entity)),
        )
        .with_destination_value_tokens([
            scope.value_token(encode_option_debug(right_value.as_ref())),
            scope.value_token(encode_option_debug(left_value.as_ref())),
        ])
}

fn ruin<S, D>(
    scope: MoveTabuScope,
    slot: &RuntimeScalarSlot<S>,
    director: &D,
    entities: &[usize],
    recreate_heuristic_type: solverforge_config::RecreateHeuristicType,
) -> MoveTabuSignature
where
    S: PlanningSolution,
    D: Director<S>,
{
    let heuristic = match recreate_heuristic_type {
        solverforge_config::RecreateHeuristicType::FirstFit => hash_str("first_fit"),
        solverforge_config::RecreateHeuristicType::CheapestInsertion => {
            hash_str("cheapest_insertion")
        }
    };
    let variable = hash_str(slot.variable_name());
    let mut move_id = smallvec![
        hash_str("ruin_recreate"),
        encode_usize(slot.descriptor_index()),
        variable,
        heuristic,
        encode_usize(entities.len()),
    ];
    for &entity in entities {
        move_id.push(encode_usize(entity));
        move_id.push(encode_option_usize(
            slot.current_value(director.working_solution(), entity),
        ));
    }
    MoveTabuSignature::new(scope, move_id.clone(), move_id).with_entity_tokens(
        entities
            .iter()
            .copied()
            .map(encode_usize)
            .map(|entity| scope.entity_token(entity)),
    )
}
