/* Zero-erasure cross-bi-constraint stream for cross-entity join patterns.

A `Bi` operates on pairs of entities from different
collections, such as (Shift, Employee) joins. All type information is
preserved at compile time - no Arc, no dyn, fully monomorphized.
*/

mod base;
mod complemented_grouped;
mod grouped;
mod weighting;

pub use base::Bi;
pub use complemented_grouped::{ComplementedGrouped, ComplementedGroupedBuilder};
pub use grouped::{Grouped, GroupedBuilder};
pub use weighting::Builder;
