//! Move selector decorators for filtering, limiting, and transforming moves.
//!
//! Decorators wrap an inner [`MoveSelector`] to modify its behavior without
//! changing the move type. All decorators preserve the zero-erasure architecture.
//!
//! - [`CartesianProductMoveSelector`] - combines two selectors into composite moves
//! - [`CachingMoveSelector`] - caches moves to avoid recomputation
//! - [`FilteringMoveSelector`] - filters moves by predicate
//! - [`ProbabilityMoveSelector`] - randomly selects moves with probability
//! - [`SelectedCountLimitMoveSelector`] - limits selected move count
//! - [`ShufflingMoveSelector`] - randomizes move order
//! - [`SortingMoveSelector`] - orders moves by key function
//! - [`UnionMoveSelector`] - chains two selectors sequentially

mod caching;
mod cartesian_product;
mod count_limit;
mod filtering;
mod probability;
mod shuffling;
mod sorting;
mod union;

pub use caching::CachingMoveSelector;
pub use cartesian_product::CartesianProductMoveSelector;
pub use count_limit::SelectedCountLimitMoveSelector;
pub use filtering::FilteringMoveSelector;
pub use probability::ProbabilityMoveSelector;
pub use shuffling::ShufflingMoveSelector;
pub use sorting::SortingMoveSelector;
pub use union::UnionMoveSelector;
