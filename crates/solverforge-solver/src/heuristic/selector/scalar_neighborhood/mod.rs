//! Canonical typed/dynamic scalar neighborhood leaf kernel.
//!
//! A leaf owns one frozen `RuntimeScalarSlot` and one family spec. It produces
//! owned recipes through one streamed cursor; typed public selectors only
//! project move payloads and never enumerate candidates independently.

mod adapter;
mod cursor;
mod r#move;
mod spec;

#[cfg(test)]
mod tests;

pub(crate) use adapter::{
    dynamic_slot, emit_dynamic_scalar_change_move, emit_dynamic_scalar_swap_move,
    RuntimeScalarFacadeCursor,
};
pub(crate) use cursor::ScalarNeighborhoodStreamState;
pub(crate) use cursor::{RuntimeScalarNeighborhoodCursor, ScalarNeighborhoodLeaf};
pub(crate) use r#move::RuntimeScalarMove;
pub use spec::ScalarNeighborhoodBindingError;
pub(crate) use spec::{RuntimeScalarRecipe, ScalarNeighborhoodKind, ScalarNeighborhoodSpec};
