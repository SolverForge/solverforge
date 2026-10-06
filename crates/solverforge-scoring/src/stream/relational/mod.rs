/* Typed relational row machinery for the common operator tree.

Stable generational identities, dense swap_remove storage, borrowed row
shapes, concrete index strategies, and the operator protocol. Scoring
terminals live under `constraint::relational`. Nothing here introduces a
public erased graph API: every type is concrete and monomorphized.
*/

mod handle_map;
mod identity;
pub(crate) mod index;
pub(crate) mod leaf_collector;
pub mod operator;
mod row;
mod storage;
pub(crate) mod view_plan;

pub(crate) use handle_map::HandleMap;
#[doc(hidden)]
pub use identity::RowHandle;
pub(crate) use identity::{BindingId, JoinedIdentity};
#[allow(unused_imports)]
pub(crate) use row::Row;
pub use row::{Concat, Leaf};
pub(crate) use storage::DenseRowStore;
