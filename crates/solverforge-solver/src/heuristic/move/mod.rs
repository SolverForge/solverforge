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
//! - `PillarChangeMove<S, V>` - changes multiple entities with same value
//! - `PillarSwapMove<S, V>` - swaps between two pillars
//! - `ListChangeMove<S, V>` - relocates an element in a list variable
//! - `ListSwapMove<S, V>` - swaps two elements in list variables
//! - `SubListChangeMove<S, V>` - relocates a contiguous sublist
//! - `SubListSwapMove<S, V>` - swaps two contiguous sublists
//! - `ListReverseMove<S, V>` - reverses a segment (2-opt for TSP)
//! - `RuinMove<S, V>` - unassigns multiple entities (for Large Neighborhood Search)
//! - `ListRuinMove<S, V>` - removes elements from a list (for LNS on list variables)
//! - `CompositeMove<S, M1, M2>` - combines two moves in sequence
//!
//! Undo is handled by `RecordingScoreDirector`, not by moves returning undo data.
//!
//! # Arena Allocation
//!
//! Use `MoveArena<M>` for O(1) per-step cleanup. Call `reset()` at each step
//! instead of allocating a new Vec.

mod arena;
mod change;
mod composite;
mod k_opt;
pub mod k_opt_reconnection;
mod list_change;
mod list_reverse;
mod list_ruin;
mod list_swap;
mod pillar_change;
mod pillar_swap;
mod ruin;
mod sublist_change;
mod sublist_swap;
mod swap;

use std::fmt::Debug;

use solverforge_core::domain::PlanningSolution;
use solverforge_scoring::ScoreDirector;

pub use arena::MoveArena;
pub use change::ChangeMove;
pub use composite::CompositeMove;
pub use k_opt::{CutPoint, KOptMove};
pub use list_change::ListChangeMove;
pub use list_reverse::ListReverseMove;
pub use list_ruin::ListRuinMove;
pub use list_swap::ListSwapMove;
pub use pillar_change::PillarChangeMove;
pub use pillar_swap::PillarSwapMove;
pub use ruin::RuinMove;
pub use sublist_change::SubListChangeMove;
pub use sublist_swap::SubListSwapMove;
pub use swap::SwapMove;

/// A move that modifies one or more planning variables.
///
/// Moves are fully typed for maximum performance - no boxing, no virtual dispatch.
/// Undo is handled by `RecordingScoreDirector`, not by move return values.
///
/// # Type Parameters
/// * `S` - The planning solution type
/// * `D` - The score director type
///
/// # Implementation Notes
/// - Moves should be lightweight and cloneable
/// - Use `RecordingScoreDirector` to wrap the score director for automatic undo
/// - Implement `Clone` for arena allocation support
pub trait Move<S: PlanningSolution, D: ScoreDirector<S>>: Send + Sync + Debug + Clone {
    /// Returns true if this move can be executed in the current state.
    ///
    /// A move is not doable if:
    /// - The source value equals the destination value (no change)
    /// - Required entities are pinned
    /// - The move would violate hard constraints that can be detected early
    fn is_doable(&self, score_director: &D) -> bool;

    /// Executes this move, modifying the working solution.
    ///
    /// This method modifies the planning variables through the score director.
    /// Use `RecordingScoreDirector` to enable automatic undo via `undo_changes()`.
    fn do_move(&self, score_director: &mut D);

    /// Returns the descriptor index of the entity type this move affects.
    fn descriptor_index(&self) -> usize;

    /// Returns the entity indices involved in this move.
    fn entity_indices(&self) -> &[usize];

    /// Returns the variable name this move affects.
    fn variable_name(&self) -> &str;
}
