// Tabu signatures and candidate-trace identities of runtime list moves.

use std::fmt;

use solverforge_core::domain::PlanningSolution;
use solverforge_core::score::Score;
use solverforge_scoring::Director;

use crate::heuristic::r#move::list_kernel::{
    change_candidate_trace_identity, change_tabu_signature, k_opt_tabu_signature,
    multi_swap_tabu_signature, permute_candidate_trace_identity, permute_tabu_signature,
    reverse_candidate_trace_identity, reverse_tabu_signature, ruin_tabu_signature,
    sublist_change_candidate_trace_identity, sublist_change_tabu_signature,
    sublist_swap_candidate_trace_identity, sublist_swap_tabu_signature,
    swap_candidate_trace_identity, swap_tabu_signature, ListMoveAccess,
};
use crate::heuristic::r#move::MoveTabuSignature;
use crate::stats::CandidateTraceIdentity;

use super::spec::RuntimeListRecipe;

pub(super) fn recipe_tabu_signature<S, V, D: Director<S>>(
    recipe: &RuntimeListRecipe<S, V>,
    entity_indices: &[usize],
    score_director: &D,
) -> MoveTabuSignature
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    match recipe {
        RuntimeListRecipe::Change {
            access,
            coordinates,
        } => change_tabu_signature(access, *coordinates, score_director),
        RuntimeListRecipe::Swap {
            access,
            coordinates,
        } => swap_tabu_signature(access, *coordinates, score_director),
        RuntimeListRecipe::Permute {
            access,
            coordinates,
            permutation,
            inverse_permutation,
        } => permute_tabu_signature(
            access,
            *coordinates,
            permutation,
            inverse_permutation,
            score_director,
        ),
        RuntimeListRecipe::Reverse {
            access,
            coordinates,
        } => reverse_tabu_signature(access, *coordinates, score_director),
        RuntimeListRecipe::SublistChange {
            access,
            coordinates,
        } => sublist_change_tabu_signature(access, *coordinates, score_director),
        RuntimeListRecipe::SublistSwap {
            access,
            coordinates,
        } => sublist_swap_tabu_signature(access, *coordinates, score_director),
        RuntimeListRecipe::KOpt {
            access,
            entity,
            cuts,
            reconnection,
            variable_id,
        } => k_opt_tabu_signature(
            access,
            cuts,
            reconnection,
            *variable_id,
            *entity,
            score_director,
        ),
        RuntimeListRecipe::Ruin {
            access, sources, ..
        } => ruin_tabu_signature(access, sources, entity_indices, score_director),
        RuntimeListRecipe::MultiSwap {
            access,
            coordinates,
            ..
        } => multi_swap_tabu_signature(access, coordinates, entity_indices, score_director),
    }
}

pub(super) fn recipe_candidate_trace_identity<S, V>(
    recipe: &RuntimeListRecipe<S, V>,
) -> Option<CandidateTraceIdentity>
where
    S: PlanningSolution + Clone + Send + Sync + 'static,
    S::Score: Score,
    V: Clone + PartialEq + Send + Sync + fmt::Debug + 'static,
{
    let identity = match recipe {
        RuntimeListRecipe::Change {
            access,
            coordinates,
        } => change_candidate_trace_identity(access, *coordinates),
        RuntimeListRecipe::Swap {
            access,
            coordinates,
        } => swap_candidate_trace_identity(access, *coordinates),
        RuntimeListRecipe::Permute {
            access,
            coordinates,
            permutation,
            ..
        } => permute_candidate_trace_identity(access, *coordinates, permutation),
        RuntimeListRecipe::Reverse {
            access,
            coordinates,
        } => reverse_candidate_trace_identity(access, *coordinates),
        RuntimeListRecipe::SublistChange {
            access,
            coordinates,
        } => sublist_change_candidate_trace_identity(access, *coordinates),
        RuntimeListRecipe::SublistSwap {
            access,
            coordinates,
        } => sublist_swap_candidate_trace_identity(access, *coordinates),
        RuntimeListRecipe::KOpt {
            access,
            entity,
            cuts,
            reconnection,
            ..
        } => CandidateTraceIdentity::logical_move(
            ListMoveAccess::descriptor_index(access),
            ListMoveAccess::variable_name(access),
            "k_opt",
            cuts.iter().flat_map(|cut| [*entity, cut.position()]).chain(
                reconnection
                    .segment_order()
                    .iter()
                    .copied()
                    .map(usize::from),
            ),
        ),
        RuntimeListRecipe::Ruin {
            access, sources, ..
        } => CandidateTraceIdentity::logical_move(
            ListMoveAccess::descriptor_index(access),
            ListMoveAccess::variable_name(access),
            "list_ruin",
            sources.iter().flat_map(|(entity, indices)| {
                std::iter::once(*entity)
                    .chain(std::iter::once(indices.len()))
                    .chain(indices.iter().copied())
            }),
        ),
        RuntimeListRecipe::MultiSwap {
            access,
            coordinates,
            ..
        } => CandidateTraceIdentity::logical_move(
            ListMoveAccess::descriptor_index(access),
            ListMoveAccess::variable_name(access),
            "list_multi_swap",
            coordinates
                .iter()
                .flat_map(|&(entity, first, second)| [entity, first, second]),
        ),
    };
    Some(identity)
}
