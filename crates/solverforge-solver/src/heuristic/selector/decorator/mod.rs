/* Move selector composition used by the compiled selector pipeline.

Decorators wrap an inner [`MoveSelector`] to modify its behavior without
changing the move type. All decorators preserve the zero-erasure architecture.

- [`CartesianProductArena`] - stores moves from two selectors for pair iteration
- [`FilteringMoveSelector`] - filters moves by predicate
*/

pub(crate) mod cartesian_product;
pub(crate) mod filtering;
mod limited;
#[cfg(test)]
mod test_utils;
pub(crate) mod vec_union;

pub(crate) use limited::LimitedCursorBudget;
#[allow(unused_imports)] // Consumed by the resource-aware composed runtime cursor.
pub(crate) use vec_union::{
    resolve_union_weights, union_child_context, ResourceVecUnionMoveCursor, UnionScheduler,
    VecUnionMoveCursor,
};
