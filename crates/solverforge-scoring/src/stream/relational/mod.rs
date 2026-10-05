/* Typed relational row machinery shared by join operators and scoring terminals.

Produces reusable rows independently of scoring: stable generational
identities, dense swap_remove storage, borrowed row shapes, provenance
traversal, and delta coalescing. Terminal scoring lives under
`constraint::relational`. Neither module introduces a public erased graph
API: everything here is concrete and monomorphized.
*/

mod delta;
mod identity;
mod index;
mod join;
mod provenance;
mod row;
mod source;
mod storage;

pub(crate) use delta::{DeltaBuffer, OutputDelta};
pub(crate) use identity::{BindingId, JoinedIdentity, RowHandle};
pub(crate) use index::HashIndex;
pub(crate) use join::{DeltaKind, EquiJoin};
pub(crate) use provenance::{Participation, Provenance};
pub(crate) use row::Leaf;
pub(crate) use source::Source;
pub(crate) use storage::DenseRowStore;
