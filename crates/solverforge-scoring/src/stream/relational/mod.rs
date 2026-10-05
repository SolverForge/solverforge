/* Typed relational row machinery shared by join operators and scoring terminals.

Produces reusable rows independently of scoring: stable generational
identities, dense swap_remove storage, borrowed row shapes, provenance
traversal, and delta coalescing. Terminal scoring lives under
`constraint::relational`. Neither module introduces a public erased graph
API: everything here is concrete and monomorphized.
*/

mod chain;
mod delta;
mod handle_map;
mod identity;
pub(crate) mod index;
mod join;
pub mod operator;
mod provenance;
mod row;
mod source;
mod storage;

pub(crate) use chain::ChainedJoin;
pub(crate) use delta::{DeltaBuffer, OutputDelta};
pub(crate) use handle_map::HandleMap;
#[doc(hidden)]
pub use identity::RowHandle;
pub(crate) use identity::{BindingId, JoinedIdentity};
pub(crate) use index::HashIndex;
pub(crate) use join::{DeltaKind, EquiJoin};
pub(crate) use provenance::{Participation, Provenance};
#[allow(unused_imports)]
pub(crate) use row::Row;
pub use row::{Concat, Leaf};
pub(crate) use source::Source;
pub(crate) use storage::DenseRowStore;
