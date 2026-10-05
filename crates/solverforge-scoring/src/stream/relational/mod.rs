/* Typed relational row machinery shared by join operators and scoring terminals.

Produces reusable rows independently of scoring: stable generational
identities and dense swap_remove storage land here; borrowed row shapes,
provenance traversal, and delta coalescing arrive with the P2 operators
that consume them. Terminal scoring lives under `constraint::relational`.
Neither module introduces a public erased graph API: everything here is
concrete and monomorphized.
*/

mod identity;
mod storage;

pub(crate) use identity::RowHandle;
pub(crate) use storage::DenseRowStore;
