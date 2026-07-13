//! One non-routed runtime list-neighborhood kernel for typed and dynamic slots.
//!
//! The compiler has already frozen the list family, exact configuration, and
//! declaration-order `RuntimeListSlot` carriers. This module consumes that
//! immutable input through the shared list kernels; it never rediscovers a
//! model, substitutes a static selector, or touches construction execution.

mod compiled_leaf;
mod cursor;
mod emission;
mod r#move;
mod move_access;
mod recipe;
mod spec;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use compiled_leaf::RuntimeListNeighborhoodLeafError;
pub(crate) use compiled_leaf::{CompiledListNeighborhoodLeafAdapter, RuntimeListNeighborhoodLeaf};
pub(crate) use cursor::{
    RuntimeListNeighborhoodCursor, RuntimeListNeighborhoodSelector,
    RuntimeListNeighborhoodStreamState,
};
pub(crate) use r#move::RuntimeListMove;
pub(crate) use spec::{
    RuntimeListNeighborhoodPlan, RuntimeListNeighborhoodPlanError, RuntimeListRecipe,
};
