//! Heuristic components for solving.
//!
//! Use `MoveImpl` for moves and `MoveSelectorImpl` for selectors.

pub(crate) mod r#move;
pub(crate) mod selector;

pub use r#move::{Move, MoveArena, MoveImpl};
pub use selector::{MoveSelector, MoveSelectorImpl};
