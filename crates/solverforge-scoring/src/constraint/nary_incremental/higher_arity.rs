/* Higher-arity incremental constraint macros for tri/quad/penta arities.

The arity-specific macro definitions now live in dedicated files so this
module root stays limited to module wiring and re-exports.
*/

#[macro_use]
mod shared;
#[macro_use]
mod penta;
#[macro_use]
mod quad;
#[macro_use]
mod tri;

pub use penta::impl_incremental_penta_constraint;
pub use quad::impl_incremental_quad_constraint;
pub use tri::impl_incremental_tri_constraint;
