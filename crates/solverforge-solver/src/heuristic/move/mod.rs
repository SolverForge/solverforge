/* Move system for modifying planning solutions.

Moves are the fundamental operations that modify planning variables during
solving. The solver explores the solution space by applying different moves
and evaluating their impact on the score.

# Architecture

All moves are fully monomorphized with inline value storage for maximum performance:
- `ChangeMove<S, V>` - assigns a value to a variable
- `SwapMove<S, V>` - swaps values between two entities
- `CompositeMove<'a, S, M1, M2>` - applies two moves by reference
- `SequentialCompositeMove<S, M>` - applies two owned moves in sequence
- `PillarChangeMove<S, V>` - changes multiple entities with same value
- `PillarSwapMove<S, V>` - swaps between two pillars
- `ListChangeMove<S, V>` - relocates an element in a list variable
- `ListSwapMove<S, V>` - swaps two elements in list variables
- `SublistChangeMove<S, V>` - relocates a contiguous sublist
- `SublistSwapMove<S, V>` - swaps two contiguous sublists
- `ListReverseMove<S, V>` - reverses a segment (2-opt for TSP)
- `RuinMove<S, V>` - unassigns multiple entities (for Large Neighborhood Search)
- `ListRuinMove<S, V>` - removes elements from a list (for LNS on list variables)

Each move returns typed undo data from `do_move` and restores itself through
`undo_move`; speculative evaluation does not use boxed callbacks.

# Arena Allocation

Use `MoveArena<M>` for O(1) per-step cleanup. Call `reset()` at each step
instead of allocating a new Vec.

# Zero-Erasure Design

Moves are NEVER cloned. Ownership transfers via arena indices:

```
use solverforge_solver::heuristic::MoveArena;

// Simple move type for demonstration
struct SimpleMove { value: i32 }

let mut arena: MoveArena<SimpleMove> = MoveArena::new();

// Store moves - track indices manually
arena.push(SimpleMove { value: 1 }); // index 0
arena.push(SimpleMove { value: 2 }); // index 1

// Take ownership from arena when picking
let selected = arena.take(0);
assert_eq!(selected.value, 1);

// Reset clears arena for next step
arena.reset();
```
*/

/// Inlined undo journal for short list edit sets (K-opt route restorations
/// and similar arbitrary-length undo payloads). Capacity is inline up to 24
/// elements; longer routes spill to the heap and rely on the move's
/// per-trial journal pool so retained capacity is reused across trials.
pub(crate) type ListUndoJournal<T> = smallvec::SmallVec<[T; 24]>;

pub(crate) mod arena;
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

pub(crate) use arena::MoveArena;
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
