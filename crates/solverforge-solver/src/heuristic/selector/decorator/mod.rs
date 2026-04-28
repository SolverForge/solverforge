/* Move selector decorators for filtering and ordering moves.

Decorators wrap an inner [`MoveSelector`] to modify its behavior without
changing the move type. All decorators preserve the zero-erasure architecture.

- [`CartesianProductArena`] - stores moves from two selectors for pair iteration
- [`FilteringMoveSelector`] - filters moves by predicate
- [`ProbabilityMoveSelector`] - randomly selects moves with probability
- [`ShufflingMoveSelector`] - randomizes move order
- [`SortingMoveSelector`] - orders moves by key function
- [`UnionMoveSelector`] - chains two selectors sequentially
*/

mod cartesian_product;
mod filtering;
mod indexed_cursor;
mod limited;
mod mapped_cursor;
mod probability;
mod shuffling;
mod sorting;
#[cfg(test)]
mod test_utils;
mod union;
mod vec_union;

pub(crate) use cartesian_product::CartesianProductCursor;
pub use cartesian_product::{CartesianProductArena, CartesianProductSelector};
pub use filtering::FilteringMoveSelector;
pub(crate) use limited::LimitedMoveCursor;
pub(crate) use mapped_cursor::MappedMoveCursor;
pub use probability::ProbabilityMoveSelector;
pub use shuffling::ShufflingMoveSelector;
pub use sorting::SortingMoveSelector;
pub use union::UnionMoveSelector;
pub use vec_union::VecUnionSelector;
