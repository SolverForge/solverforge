/* Move system for modifying planning solutions.

Moves are the fundamental operations that modify planning variables during
solving. The solver explores the solution space by applying different moves
and evaluating their impact on the score.

# Architecture

All moves are fully typed with inline value storage for maximum performance:
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

Undo is handled by `RecordingDirector`, not by moves returning undo data.

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

mod arena;
mod change;
mod composite;
mod either;
mod k_opt;
pub mod k_opt_reconnection;
mod list_change;
mod list_either;
mod list_reverse;
mod list_ruin;
mod list_swap;
pub(crate) mod metadata;
mod pillar_change;
mod pillar_swap;
mod ruin;
mod ruin_recreate;
mod segment_layout;
mod sublist_change;
mod sublist_swap;
mod swap;
mod traits;

#[cfg(test)]
mod tests;

pub use arena::MoveArena;
pub use change::ChangeMove;
pub use composite::CompositeMove;
pub use composite::SequentialCompositeMove;
pub(crate) use composite::SequentialCompositeMoveRef;
pub(crate) use composite::SequentialPreviewDirector;
pub use either::ScalarMoveUnion;
pub use k_opt::{CutPoint, KOptMove};
pub use list_change::ListChangeMove;
pub use list_either::ListMoveUnion;
pub use list_reverse::ListReverseMove;
pub use list_ruin::ListRuinMove;
pub use list_swap::ListSwapMove;
pub use metadata::MoveTabuSignature;
pub use pillar_change::PillarChangeMove;
pub use pillar_swap::PillarSwapMove;
pub use ruin::RuinMove;
pub use ruin_recreate::{RuinRecreateMove, ScalarRecreateValueSource};
pub use sublist_change::SublistChangeMove;
pub use sublist_swap::SublistSwapMove;
pub use swap::SwapMove;
pub use traits::Move;
