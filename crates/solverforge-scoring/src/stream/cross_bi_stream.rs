/* Zero-erasure cross-bi-constraint stream for cross-entity join patterns.

A `CrossBiConstraintStream` operates on pairs of entities from different
collections, such as (Shift, Employee) joins. All type information is
preserved at compile time - no Arc, no dyn, fully monomorphized.
*/

mod base;
mod weighting;

pub use base::CrossBiConstraintStream;
pub use weighting::CrossBiConstraintBuilder;
