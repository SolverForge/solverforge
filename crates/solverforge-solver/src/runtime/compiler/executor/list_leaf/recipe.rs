use smallvec::SmallVec;

use crate::builder::context::RuntimeListElement;
use crate::heuristic::r#move::list_kernel::{
    multi_swap_entity_indices, ruin_entity_indices, RuinUndo,
};

use super::move_access::RuntimeListMoveAccess;
use super::spec::RuntimeListRecipe;

#[derive(Clone, Debug)]
pub(crate) enum RuntimeListMoveUndo<V> {
    None,
    Permute(Vec<RuntimeListElement<V>>),
    KOpt(Vec<RuntimeListElement<V>>),
    Ruin(RuinUndo),
}

pub(super) fn recipe_access<S, V>(
    recipe: &RuntimeListRecipe<S, V>,
) -> &RuntimeListMoveAccess<S, V> {
    match recipe {
        RuntimeListRecipe::Change { access, .. }
        | RuntimeListRecipe::Swap { access, .. }
        | RuntimeListRecipe::Permute { access, .. }
        | RuntimeListRecipe::Reverse { access, .. }
        | RuntimeListRecipe::SublistChange { access, .. }
        | RuntimeListRecipe::SublistSwap { access, .. }
        | RuntimeListRecipe::KOpt { access, .. }
        | RuntimeListRecipe::Ruin { access, .. }
        | RuntimeListRecipe::MultiSwap { access, .. } => access,
    }
}

pub(super) fn recipe_entity_indices<S, V>(
    recipe: &RuntimeListRecipe<S, V>,
) -> SmallVec<[usize; 8]> {
    let mut entities = match recipe {
        RuntimeListRecipe::Change { coordinates, .. } => {
            SmallVec::from_slice(&[coordinates.source_entity, coordinates.destination_entity])
        }
        RuntimeListRecipe::Swap { coordinates, .. } => {
            SmallVec::from_slice(&[coordinates.first_entity, coordinates.second_entity])
        }
        RuntimeListRecipe::Permute { coordinates, .. } => {
            SmallVec::from_slice(&[coordinates.entity])
        }
        RuntimeListRecipe::Reverse { coordinates, .. } => {
            SmallVec::from_slice(&[coordinates.entity])
        }
        RuntimeListRecipe::SublistChange { coordinates, .. } => SmallVec::from_slice(&[
            coordinates.source_entity_index,
            coordinates.dest_entity_index,
        ]),
        RuntimeListRecipe::SublistSwap { coordinates, .. } => SmallVec::from_slice(&[
            coordinates.first_entity_index,
            coordinates.second_entity_index,
        ]),
        RuntimeListRecipe::KOpt { entity, .. } => SmallVec::from_slice(&[*entity]),
        RuntimeListRecipe::Ruin { sources, .. } => ruin_entity_indices(sources),
        RuntimeListRecipe::MultiSwap { coordinates, .. } => {
            multi_swap_entity_indices(coordinates).into_iter().collect()
        }
    };
    entities.sort_unstable();
    entities.dedup();
    entities
}

pub(super) fn recipe_family<S, V>(recipe: &RuntimeListRecipe<S, V>) -> &'static str {
    match recipe {
        RuntimeListRecipe::Change { .. } => "change",
        RuntimeListRecipe::Swap { .. } => "swap",
        RuntimeListRecipe::Permute { .. } => "permute",
        RuntimeListRecipe::Reverse { .. } => "reverse",
        RuntimeListRecipe::SublistChange { .. } => "sublist_change",
        RuntimeListRecipe::SublistSwap { .. } => "sublist_swap",
        RuntimeListRecipe::KOpt { .. } => "k_opt",
        RuntimeListRecipe::Ruin { .. } => "ruin",
        RuntimeListRecipe::MultiSwap { .. } => "multi_swap",
    }
}
