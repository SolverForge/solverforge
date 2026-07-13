/* Move selector composition used by the compiled selector pipeline.

Decorators wrap an inner [`MoveSelector`] to modify its behavior without
changing the move type. All decorators preserve the zero-erasure architecture.

- [`CartesianProductArena`] - stores moves from two selectors for pair iteration
- [`FilteringMoveSelector`] - filters moves by predicate
*/

mod cartesian_product;
mod filtering;
mod limited;
mod mapped_cursor;
#[cfg(test)]
mod test_utils;
mod vec_union;

pub(crate) use cartesian_product::CartesianProductCursor;
pub use cartesian_product::{CartesianProductArena, CartesianProductSelector};
pub use filtering::FilteringMoveSelector;
pub(crate) use limited::LimitedMoveCursor;
pub(crate) use mapped_cursor::MappedMoveCursor;
pub use vec_union::VecUnionSelector;
#[allow(unused_imports)] // Consumed by the resource-aware composed runtime cursor.
pub(crate) use vec_union::{
    resolve_union_weights, union_child_context, union_cursor_from_opened,
    ResourceVecUnionMoveCursor, UnionScheduler, VecUnionMoveCursor,
};
