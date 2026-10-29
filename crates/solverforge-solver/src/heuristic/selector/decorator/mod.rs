/* Move selector decorators.

- `VecUnionMoveCursor` - the union cursor the compiled pipeline schedules
- `LimitedCursorBudget` - the candidate-limit budget the compiled limited
  composition applies
- `CartesianProductSelector`, `FilteringMoveSelector` - test-only oracles; the
  compiled pipeline composes Cartesian products in its own composition cursor
  and has no filtering decorator
*/

#[cfg(test)]
pub(crate) mod cartesian_product;
#[cfg(test)]
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
