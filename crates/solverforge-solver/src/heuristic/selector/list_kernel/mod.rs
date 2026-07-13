//! Canonical streamed full-list neighborhood kernels.
//!
//! Public selector facades retain their move and cursor types.  This module
//! owns the shared coordinate stream, frozen owner snapshot, and emission
//! boundary; it deliberately knows nothing about compiler lowering.

mod change;
mod emission;
mod k_opt;
mod nearby_change;
mod nearby_probe;
mod nearby_swap;
mod ownership;
mod permute;
mod precedence;
mod reverse;
mod ruin;
mod ruin_emission;
mod sublist_change;
mod sublist_swap;
mod swap;
mod window_emission;

pub(crate) use change::{ChangeCursor, DYNAMIC_CHANGE_SALTS, STATIC_CHANGE_SALTS};
pub(crate) use emission::{
    ChangeEmitter, DynamicChangeEmitter, NativeChangeEmitter, NativeSwapEmitter, SwapEmitter,
};
pub(crate) use k_opt::{
    KOptCursor, KOptDistanceProbe, KOptEmitter, NativeKOptEmitter, NearbyKOptCursor,
};
pub(crate) use nearby_change::{
    NearbyChangeCursor, STATIC_NEARBY_CHANGE_ENTITY_SALT, STATIC_NEARBY_CHANGE_SOURCE_SALT,
};
pub(crate) use nearby_probe::{NativeNearbyProbe, NearbyChangeProbe, NearbySwapProbe};
pub(crate) use nearby_swap::{
    NearbySwapCursor, STATIC_NEARBY_SWAP_ENTITY_SALT, STATIC_NEARBY_SWAP_SOURCE_SALT,
};
pub(crate) use ownership::SelectedListOwners;
pub(crate) use permute::{count_list_permute_moves_for_len, factorial, PermuteCursor};
pub(crate) use precedence::{
    critical_analysis, critical_analysis_from_graph, filtered_move_count,
    filtered_multi_support_swap_count, multi_critical_ruin_count, CriticalAnalysis,
    NativePrecedenceEmitter, PrecedenceCursor, PrecedenceEmitter,
};
pub(crate) use reverse::{ReverseCursor, STATIC_REVERSE_ENTITY_SALT};
pub(crate) use ruin::{RuinCursor, RuinSourcePool};
pub(crate) use ruin_emission::{NativeRuinEmitter, RuinEmitter};
pub(crate) use sublist_change::{SublistChangeCursor, STATIC_SUBLIST_CHANGE_SALTS};
pub(crate) use sublist_swap::{SublistSwapCursor, STATIC_SUBLIST_SWAP_ENTITY_SALT};
pub(crate) use swap::{SwapCursor, STATIC_SWAP_SALTS};
pub(crate) use window_emission::{
    NativeReverseEmitter, NativeWindowEmitter, PermuteEmitter, ReverseEmitter,
    SublistChangeEmitter, SublistSwapEmitter,
};
