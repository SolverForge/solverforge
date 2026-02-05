//! Zero-erasure filter composition for constraint streams.
//!
//! Filters are composed at compile time using nested generic types,
//! avoiding dynamic dispatch and Arc allocations.

mod adapters;
mod composition;
mod traits;
mod wrappers;

pub use adapters::{UniBiFilter, UniLeftBiFilter};
pub use composition::{AndBiFilter, AndPentaFilter, AndQuadFilter, AndTriFilter, AndUniFilter};
pub use traits::{BiFilter, PentaFilter, QuadFilter, TriFilter, UniFilter};
pub use wrappers::{FnBiFilter, FnPentaFilter, FnQuadFilter, FnTriFilter, FnUniFilter, TrueFilter};

#[cfg(test)]
mod tests;
