/* Move system for modifying planning solutions.

Moves are the fundamental operations that modify planning variables during
solving. The solver explores the solution space by applying different moves
and evaluating their impact on the score.

All moves are fully monomorphized with inline value storage. Each move returns
typed undo data from `do_move` and restores itself through `undo_move`;
speculative evaluation does not use boxed callbacks.

Moves are never cloned: selector cursors own discovered candidates behind
stable `CandidateId`s, and the selected move is taken from its cursor by
ownership.
*/

/// Inlined undo journal for short list edit sets (K-opt route restorations
/// and similar arbitrary-length undo payloads). Capacity is inline up to 24
/// elements; longer routes spill to the heap and rely on the move's
/// per-trial journal pool so retained capacity is reused across trials.
pub(crate) type ListUndoJournal<T> = smallvec::SmallVec<[T; 24]>;

pub(crate) mod change;
pub(crate) mod composite;
pub(crate) mod compound_scalar;
#[cfg(test)]
pub(crate) mod dynamic_list_change;
#[cfg(test)]
pub(crate) mod dynamic_scalar_change;
#[cfg(test)]
pub(crate) mod dynamic_scalar_swap;
pub(crate) mod k_opt;
pub(crate) mod k_opt_reconnection;
pub(crate) mod list_change;
pub(crate) mod list_kernel;
#[cfg(test)]
pub(crate) mod list_multi_swap;
pub(crate) mod list_permute;
#[cfg(test)]
pub(crate) mod list_reverse;
#[cfg(test)]
pub(crate) mod list_ruin;
pub(crate) mod list_swap;
#[cfg(test)]
pub(crate) mod list_union;
pub(crate) mod metadata;
pub(crate) mod ruin;
pub(crate) mod runtime_compound;
mod segment_layout;
pub(crate) mod sublist_change;
pub(crate) mod sublist_swap;
pub(crate) mod traits;

#[cfg(test)]
mod tests;

pub(crate) use change::ChangeMove;
pub(crate) use composite::SequentialCompositeMove;
pub(crate) use composite::SequentialCompositeMoveRef;
pub(crate) use compound_scalar::{CompoundScalarEdit, CompoundScalarMove};
#[cfg(test)]
pub(crate) use dynamic_list_change::DynamicListChangeMove;
#[cfg(test)]
pub(crate) use dynamic_scalar_change::DynamicScalarChangeMove;
#[cfg(test)]
pub(crate) use dynamic_scalar_swap::DynamicScalarSwapMove;
pub(crate) use k_opt::{CutPoint, KOptMove};
pub(crate) use list_change::ListChangeMove;
#[cfg(test)]
pub(crate) use list_multi_swap::ListMultiSwapMove;
pub(crate) use list_permute::{ListPermuteMove, MAX_LIST_PERMUTE_WINDOW_SIZE};
#[cfg(test)]
pub(crate) use list_reverse::ListReverseMove;
#[cfg(test)]
pub(crate) use list_ruin::ListRuinMove;
pub(crate) use list_swap::ListSwapMove;
#[cfg(test)]
pub(crate) use list_union::ListMoveUnion;
pub(crate) use metadata::MoveTabuSignature;
pub(crate) use ruin::RuinMove;
#[cfg(test)]
pub(crate) use runtime_compound::{
    reset_runtime_compound_move_clone_count, runtime_compound_move_clone_count,
};
pub(crate) use runtime_compound::{RuntimeCompoundMove, RuntimeCompoundMoveKind};
pub(crate) use segment_layout::{SegmentRelocationCoords, SegmentSwapCoords};
pub(crate) use sublist_change::SublistChangeMove;
pub(crate) use sublist_swap::SublistSwapMove;
pub(crate) use traits::{Move, MoveAffectedEntity};
