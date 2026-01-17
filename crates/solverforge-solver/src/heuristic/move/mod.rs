//! Move system for modifying planning solutions.
//!
//! Moves are the fundamental operations that modify planning variables during
//! solving. The solver explores the solution space by applying different moves
//! and evaluating their impact on the score.
//!
//! # Architecture
//!
//! All moves are fully typed with inline value storage for maximum performance:
//! - `ChangeMove<S, V>` - assigns a value to a variable
//! - `SwapMove<S, V>` - swaps values between two entities
//! - `CompositeMove<'a, S, M1, M2>` - applies two moves by reference
//! - `PillarChangeMove<S, V>` - changes multiple entities with same value
//! - `PillarSwapMove<S, V>` - swaps between two pillars
//! - `ListChangeMove<S, V>` - relocates an element in a list variable
//! - `ListSwapMove<S, V>` - swaps two elements in list variables
//! - `SubListChangeMove<S, V>` - relocates a contiguous sublist
//! - `SubListSwapMove<S, V>` - swaps two contiguous sublists
//! - `ListReverseMove<S, V>` - reverses a segment (2-opt for TSP)
//! - `RuinMove<S, V>` - unassigns multiple entities (for Large Neighborhood Search)
//! - `ListRuinMove<S, V>` - removes elements from a list (for LNS on list variables)
//!
//! Undo is handled by `RecordingScoreDirector`, not by moves returning undo data.
//!
//! # Arena Allocation
//!
//! Use `MoveArena<M>` for O(1) per-step cleanup. Call `reset()` at each step
//! instead of allocating a new Vec.
//!
//! # Zero-Erasure Design
//!
//! Moves are NEVER cloned. Ownership transfers via arena indices:
//!
//! ```
//! use solverforge_solver::MoveArena;
//!
//! // Simple move type for demonstration
//! struct SimpleMove { value: i32 }
//!
//! let mut arena: MoveArena<SimpleMove> = MoveArena::new();
//!
//! // Store moves - track indices manually
//! arena.push(SimpleMove { value: 1 }); // index 0
//! arena.push(SimpleMove { value: 2 }); // index 1
//!
//! // Take ownership from arena when picking
//! let selected = arena.take(0);
//! assert_eq!(selected.value, 1);
//!
//! // Reset clears arena for next step
//! arena.reset();
//! ```

pub(crate) mod arena;
pub(crate) mod change;
pub(crate) mod composite;
pub(crate) mod k_opt;
pub(crate) mod k_opt_reconnection;
pub(crate) mod list_change;
pub(crate) mod list_reverse;
pub(crate) mod list_ruin;
pub(crate) mod list_swap;
pub(crate) mod move_impl;
pub(crate) mod pillar_change;
pub(crate) mod pillar_swap;
pub(crate) mod ruin;
pub(crate) mod sublist_change;
pub(crate) mod sublist_swap;
pub(crate) mod swap;
pub(crate) mod traits;

pub use arena::MoveArena;
pub use move_impl::MoveImpl;
pub use traits::Move;
