//! Shared mutation primitives for list-neighborhood moves.
//!
//! Public static move structs, descriptor-resolved dynamic moves, and the
//! eventual runtime list-slot carrier delegate here. This module has no
//! selector order or compiler lowering logic.

mod access;
mod change;
mod k_opt;
mod multi_swap;
mod permute;
mod range_access;
mod range_static;
mod reverse;
mod ruin;
mod ruin_access;
mod sublist_change;
mod sublist_swap;
mod swap;

pub(crate) use access::{
    ListChangeAccess, ListMoveAccess, ListSwapAccess, StaticListChangeAccess, StaticListSwapAccess,
};
pub(crate) use change::{
    change_candidate_trace_identity, change_do_move, change_is_doable, change_tabu_signature,
    change_undo_move, ChangeCoordinates, ChangeValueTransfer,
};
pub(crate) use k_opt::{k_opt_do_move, k_opt_is_doable, k_opt_tabu_signature, k_opt_undo_move};
pub(crate) use multi_swap::{
    multi_swap_do_move, multi_swap_entity_indices, multi_swap_is_doable, multi_swap_tabu_signature,
    multi_swap_undo_move, MultiSwapCoordinates,
};
pub(crate) use permute::{
    permute_candidate_trace_identity, permute_do_move, permute_is_doable, permute_tabu_signature,
    permute_undo_move, PermuteCoordinates,
};
pub(crate) use range_access::{ListRangeAccess, ListReverseAccess, ListWindowAccess};
pub(crate) use range_static::{StaticListReverseAccess, StaticListWindowAccess};
pub(crate) use reverse::{
    reverse_candidate_trace_identity, reverse_do_move, reverse_is_doable, reverse_tabu_signature,
    ReverseCoordinates,
};
#[cfg(test)]
pub(crate) use ruin::final_positions_after_insertions;
pub(crate) use ruin::{
    merged_ruin_sources, ruin_count, ruin_do_move, ruin_entity_indices, ruin_is_doable,
    ruin_tabu_signature, ruin_undo_move, single_ruin_source, RuinSources, RuinUndo,
    RuinValueTransfer,
};
pub(crate) use ruin_access::{ListRuinAccess, StaticListRuinAccess};
pub(crate) use sublist_change::{
    sublist_change_candidate_trace_identity, sublist_change_do_move, sublist_change_is_doable,
    sublist_change_tabu_signature, sublist_change_undo_move,
};
pub(crate) use sublist_swap::{
    sublist_swap_candidate_trace_identity, sublist_swap_do_move, sublist_swap_is_doable,
    sublist_swap_tabu_signature, sublist_swap_undo_move,
};
pub(crate) use swap::{
    swap_candidate_trace_identity, swap_do_move, swap_is_doable, swap_tabu_signature,
    SwapCoordinates,
};
